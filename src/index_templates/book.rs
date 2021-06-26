use serde_json::{json, Value};

pub fn template() -> Value {
    json!({
        "index_patterns": ["book-*"],
        "template": {
            "settings": {
                "refresh_interval": "300s",
                "analysis": {
                    "analyzer": {
                        "nori-default": {
                            "type": "custom",
                            "tokenizer": "tokenizer_discard_puncuation_false",
                            "filter": [
                                "part_of_speech_stop_sp",
                                "nori_number",
                                "nori_readingform"
                            ]
                        },
                    },
                    "tokenizer": {
                        "tokenizer_discard_puncuation_false": {
                            "type": "nori_tokenizer",
                            "discard_punctuation": "false"
                        }
                    },
                    "filter": {
                        "part_of_speech_stop_sp": {
                            "type": "nori_part_of_speech",
                            "stoptags": ["SP"]
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "title": {
                        "type": "text",
                        "fields": {
                            "nori": {
                                "type": "text",
                                "analyzer": "nori-default"
                            }
                        }
                    },
                    "authors": {
                        "type": "text",
                        "fields": {
                            "nori": {
                                "type": "text",
                                "analyzer": "nori-default"
                            }
                        }
                    },
                    "publisher": {
                        "type": "text",
                        "fields": {
                            "nori": {
                                "type": "text",
                                "analyzer": "nori-default"
                            }
                        }
                    },
                    "publicationYear": {
                        "type": "keyword"
                    },
                    "isbn": {
                        "type": "keyword"
                    },
                    "setIsbn": {
                        "type": "keyword"
                    },
                    "additionSymbol": {
                        "type": "keyword"
                    },
                    "vol": {
                        "type": "keyword"
                    },
                    "kdc": {
                        "type": "keyword"
                    },
                    "bookCount": {
                        "type": "integer"
                    },
                    "loanCount": {
                        "type": "integer"
                    },
                    "regDate": {
                        "type": "date",
                        "format": "yyyy-MM-dd",
                    },
                    "libCode": {
                        "type": "keyword"
                    },
                },
            },
        },
        "version": 1,
        "_meta": {
            "description": "Index template for book"
        }
    })
}
