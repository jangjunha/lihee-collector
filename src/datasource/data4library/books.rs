use bytes::Buf;
use encoding_rs::EUC_KR;
use encoding_rs_io::DecodeReaderBytesBuilder;
use kuchiki::{parse_html, traits::*};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize)]
pub struct Record {
    pub num: i32,
    pub title: String,
    pub authors: String,
    pub publisher: String,
    pub publication_year: String,
    pub isbn: String,
    pub set_isbn: String,
    pub addition_symbol: String,
    pub vol: String,
    pub kdc: String,
    pub book_count: i32,
    pub loan_count: i32,
    pub reg_date: String,
}

#[derive(Debug, Deserialize)]
struct LibraryResult {
    rows: Vec<LibraryRow>,
}

#[derive(Debug, Deserialize)]
struct LibraryRow {
    id: u32,
    cell: Vec<String>,
}

#[derive(Debug)]
pub enum GetBookError {
    NoLibCode,
    NoLink,
    NoUrl,
    LibCodeParseError,
}

impl Display for GetBookError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "GetBookError")
    }
}

impl Error for GetBookError {}

pub async fn libraries(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let res = client
        .post("https://www.data4library.kr/srchLibs")
        .form(&[("region", "11"), ("dtl_region", "A"), ("libType", "")]) // Seoul
        .send()
        .await?
        .json::<LibraryResult>()
        .await?;
    Ok(res.rows.iter().map(|l| l.id.to_string()).collect())
}

// NOTE: API는 서버 부하가 클 것 같아서 소장도서는 웹페이지에서 CSV를 다운받아서 사용합니다.
// NOTE: API로 제공되는 lib_code와 웹페이지에서 사용하는 lib_code가 다름에 유의합니다.
pub async fn books(
    client: &Client,
    page_lib_code: &str,
) -> Result<(String, Vec<Record>), Box<dyn std::error::Error>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"libCode=(?P<lib_code>\d+)").unwrap();
    }
    let page_resp = client
        .get("https://www.data4library.kr/openDataV")
        .query(&[("libcode", page_lib_code), ("pageSize", "1")])
        .send()
        .await?;
    let html = page_resp.text().await?;
    let document = parse_html().one(html);
    if let Ok(dd) = document.select_first(".right_linfo dl dd") {
        let api_url = dd.text_contents();
        let lib_code = RE
            .captures(&api_url)
            .and_then(|cap| cap.name("lib_code").map(|id| id.as_str()));
        if let Some(lib_code) = lib_code {
            if let Ok(link) = document.select_first(".download_link.text_type") {
                let attrs = link.attributes.borrow();
                if let Some(path) = attrs.get("url") {
                    let url = format!("https://www.data4library.kr{}", path);
                    let bytes = client
                        .get(&url)
                        .header("User-Agent", "")
                        .send()
                        .await?
                        .bytes()
                        .await?;
                    let transcoded = DecodeReaderBytesBuilder::new()
                        .encoding(Some(EUC_KR))
                        .build(bytes.reader());
                    let mut reader = csv::ReaderBuilder::new().from_reader(transcoded);
                    let mut records = vec![];
                    for record in reader.records() {
                        let raw = record?;
                        let record: Record = raw.deserialize(None)?;
                        if record.isbn.is_empty() {
                            continue;
                        }
                        records.push(record);
                    }
                    Ok((lib_code.to_owned(), records))
                } else {
                    Err(Box::new(GetBookError::NoUrl))
                }
            } else {
                Err(Box::new(GetBookError::NoLink))
            }
        } else {
            Err(Box::new(GetBookError::LibCodeParseError))
        }
    } else {
        Err(Box::new(GetBookError::NoLibCode))
    }
}
