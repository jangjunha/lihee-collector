use serde_json::{json, Value};

pub fn template() -> Value {
    json!({
        "index_patterns": ["library-*"],
        "template": {
            "settings": {
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
                    "libCode": {
                        "type": "keyword"
                    },
                    "libName": {
                        "type": "text",
                        "fields": {
                            "nori": {
                                "type": "text",
                                "analyzer": "nori-default"
                            }
                        }
                    },
                    "address": {
                        "type": "keyword",
                        "fields": {
                            "nori": {
                                "type": "text",
                                "analyzer": "nori-default"
                            }
                        }
                    },
                    "location": {
                        "type": "geo_point"
                    },
                    "tel": {
                        "type": "keyword"
                    },
                    "fax": {
                        "type": "keyword"
                    },
                    "homepage": {
                        "type": "keyword"
                    },
                    "BookCount": {
                        "type": "long"
                    },
                    "operatingTime": {
                        "type": "keyword"
                    },
                    "closed": {
                        "type": "keyword"
                    }
                }
            }
        },
        "version": 1,
        "_meta": {
            "description": "Index template for library"
        }
    })
}
