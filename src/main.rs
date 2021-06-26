mod datasource;
mod index_templates;

use crate::datasource::data4library::books::Record;
use crate::datasource::data4library::libraries::Lib;
use backoff::{backoff::Backoff, future::retry_notify, Error as BackoffError, ExponentialBackoff};
use chrono::{FixedOffset, Utc};
use elasticsearch::CountParts;
use elasticsearch::{
    http::transport::Transport,
    indices::{IndicesCreateParts, IndicesPutIndexTemplateParts},
    params::Refresh,
    BulkOperation, BulkParts, Elasticsearch,
};
use itertools::Itertools;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

async fn put_templates(client: &Elasticsearch) -> Result<(), Box<dyn std::error::Error>> {
    let res = client
        .indices()
        .put_index_template(IndicesPutIndexTemplateParts::Name("library"))
        .body(&index_templates::library_template())
        .send()
        .await?;
    assert_eq!(res.status_code(), 200);
    let res = client
        .indices()
        .put_index_template(IndicesPutIndexTemplateParts::Name("book"))
        .body(&index_templates::book_template())
        .send()
        .await?;
    assert_eq!(res.status_code(), 200);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct CountResponse {
    count: u64,
}

#[derive(Debug, Deserialize)]
struct BulkResponse {
    took: i32,
    errors: bool,
    items: Vec<BulkResponseItem>,
}

#[derive(Debug, Deserialize)]
enum BulkResponseItem {
    #[serde(rename = "create")]
    Create(BulkActionContent),
    #[serde(rename = "delete")]
    Delete(BulkActionContent),
    #[serde(rename = "index")]
    Index(BulkActionContent),
    #[serde(rename = "update")]
    Update(BulkActionContent),
}

impl BulkResponseItem {
    fn content(&self) -> &BulkActionContent {
        match self {
            Self::Create(c) | Self::Delete(c) | Self::Index(c) | Self::Update(c) => &c,
        }
    }
}

#[derive(Debug, Deserialize)]
struct BulkActionContent {
    #[serde(rename = "_index")]
    index: String,
    #[serde(rename = "_id")]
    id: String,
    status: i16,
}

async fn save_libraries(
    client: &Elasticsearch,
    libraries: impl Iterator<Item = Lib>,
    skip_if_exists: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let kst = FixedOffset::east(9 * 3600);
    let today = Utc::today().with_timezone(&kst);
    let index_name = format!("library-{}", today.format("%Y-%m-%d"));

    if skip_if_exists {
        let res_cnt = client
            .count(CountParts::Index(&[&index_name]))
            .send()
            .await?
            .json::<CountResponse>()
            .await?;
        if res_cnt.count > 0 {
            return Ok(vec![]);
        }
    }

    let res_create_index = client
        .indices()
        .create(IndicesCreateParts::Index(&index_name))
        .send()
        .await?;
    assert_eq!(res_create_index.status_code(), 200);

    let mut body: Vec<BulkOperation<Value>> = vec![];
    for lib in libraries {
        if let Some(code) = lib.code {
            let op = BulkOperation::index(json!({
                "libCode": code,
                "libName": lib.name,
                "address": lib.address,
                "location": {
                    "lat": lib.latitude,
                    "lon": lib.longitude,
                },
                "tel": lib.tel,
                "fax": lib.fax,
                "homepage": lib.homepage,
                "BookCount": lib.book_count,
                "operatingTime": lib.operating_time,
                "closed": lib.closed,
            }))
            .id(code)
            .into();
            body.push(op);
        }
    }
    let res_insert = client
        .bulk(BulkParts::Index(&index_name))
        .body(body)
        .refresh(Refresh::WaitFor)
        .send()
        .await?;
    assert_eq!(res_insert.status_code(), 200);
    let res = res_insert.json::<BulkResponse>().await?;
    Ok(res
        .items
        .into_iter()
        .map(|item| item.content().id.clone())
        .collect())
}

fn backoff_default() -> ExponentialBackoff {
    let mut eb = ExponentialBackoff {
        current_interval: Duration::from_secs(15),
        initial_interval: Duration::from_secs(15),
        randomization_factor: 0.5,
        multiplier: 1.5,
        max_interval: Duration::from_secs(900),
        max_elapsed_time: Some(Duration::from_secs(1800)),
        ..ExponentialBackoff::default()
    };
    eb.reset();
    eb
}

async fn save_books(
    client: &Elasticsearch,
    date: &chrono::Date<chrono::FixedOffset>,
    lib_code: &str,
    books: impl Iterator<Item = Record>,
    skip_if_exists: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let index_name = format!("book-{}", date.format("%Y-%m-%d"));

    if skip_if_exists {
        let res_cnt = client
            .count(CountParts::Index(&[&index_name]))
            .body(json!({
                "query": {
                    "match": {
                        "libCode": lib_code,
                    },
                },
            }))
            .send()
            .await?
            .json::<CountResponse>()
            .await?;
        if res_cnt.count > 0 {
            return Ok(0);
        }
    }

    client
        .indices()
        .create(IndicesCreateParts::Index(&index_name))
        .send()
        .await?;

    let books = books.map(|book| {
        (
            format!("{}-{}", lib_code, book.isbn),
            json!({
                "title": book.title,
                "authors": book.authors,
                "publisher": book.publisher,
                "publicationYear": book.publication_year,
                "isbn": book.isbn,
                "setIsbn": book.set_isbn,
                "additionSymbol": book.addition_symbol,
                "vol": book.vol,
                "kdc": book.kdc,
                "bookCount": book.book_count,
                "loanCount": book.loan_count,
                "regDate": book.reg_date,
                "libCode": lib_code,
            }),
        )
    });

    let mut count = 0;
    for chunk in books.chunks(30000).into_iter() {
        let books: Vec<(String, Value)> = chunk.collect();
        let res_insert = retry_notify(
            backoff_default(),
            || async {
                client
                    .bulk(BulkParts::Index(&index_name))
                    .body(
                        books
                            .iter()
                            .map(|(id, book)| BulkOperation::index(book).id(id).into())
                            .collect::<Vec<BulkOperation<&Value>>>(),
                    )
                    .refresh(Refresh::False)
                    .send()
                    .await?
                    .error_for_status_code()
                    .map_err(BackoffError::Transient)
            },
            |e, dur| println!("Error happened at {:?}: {}", dur, e),
        )
        .await?;
        let res = res_insert.json::<BulkResponse>().await?;
        count += res.items.len();
    }
    Ok(count)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    static ES_HOST: &'static str = env!("ES_HOST", "ES_HOST required");
    static AUTH_KEY: &'static str = env!("AUTH_KEY", "AUTH_KEY required");
    let kst = FixedOffset::east(9 * 3600);
    let today = Utc::today().with_timezone(&kst);
    let transport = Transport::single_node(ES_HOST)?;
    let es = Elasticsearch::new(transport);
    let client = Client::new();

    put_templates(&es).await?;

    let libraries = datasource::data4library::libraries::libraries(&client, AUTH_KEY).await?;
    save_libraries(&es, libraries, true).await?;
    log::info!("[root] Libraries saved");

    for page_lib_code in datasource::data4library::books::libraries(&client).await? {
        if let Ok((lib_code, books)) =
            datasource::data4library::books::books(&client, &page_lib_code).await
        {
            let cnt = save_books(&es, &today, &lib_code, books.into_iter(), true).await?;
            log::info!("[root] {} books saved for {}", cnt, &lib_code);
        }
    }

    Ok(())
}
