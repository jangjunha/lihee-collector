use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Body {
    response: Response,
}

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "numFound")]
    num_found: i32,
    #[serde(rename = "resultNum")]
    result_num: i32,
    libs: Vec<LibResult>,
}

#[derive(Debug, Deserialize)]
struct LibResult {
    lib: Lib,
}

#[derive(Debug, Deserialize)]
pub struct Lib {
    #[serde(rename = "libCode")]
    pub code: Option<String>,
    #[serde(rename = "libName")]
    pub name: String,
    pub address: String,
    pub tel: String,
    pub fax: Option<String>,
    pub latitude: String,
    pub longitude: String,
    pub homepage: Option<String>,
    pub closed: Option<String>,
    #[serde(rename = "operatingTime")]
    pub operating_time: Option<String>,
    #[serde(rename = "BookCount")]
    pub book_count: String,
}

pub async fn libraries(
    client: &Client,
    auth_key: &str,
) -> Result<impl Iterator<Item = Lib>, Box<dyn std::error::Error>> {
    let resp = client
        .get("http://data4library.kr/api/libSrch")
        .query(&[
            ("authKey", auth_key),
            ("pageSize", "1000000"),
            ("format", "json"),
        ])
        .send()
        .await?;
    let res = resp.json::<Body>().await?;
    assert_eq!(res.response.num_found, res.response.result_num);
    Ok(res.response.libs.into_iter().map(|e| e.lib))
}
