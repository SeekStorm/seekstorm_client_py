
<img src="assets/logo.png" width="450" alt="Logo"><br>
**Python client** for the **SeekStorm vector & lexical search server**.

SeekStorm is open source licensed under the [Apache License 2.0](https://github.com/SeekStorm/SeekStorm?tab=Apache-2.0-1-ov-file#readme)

## SeekStorm REST client (Python)
[![Crates.io](https://img.shields.io/crates/v/seekstorm_client_py.svg)](https://crates.io/crates/seekstorm_client_py)
[![Downloads](https://img.shields.io/crates/d/seekstorm_client_py.svg?style=flat-square)](https://crates.io/crates/seekstorm_client_py)
[![Documentation](https://docs.rs/seekstorm_client_py/badge.svg)](https://docs.rs/seekstorm_client_py)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/SeekStorm/SeekStorm?tab=Apache-2.0-1-ov-file#readme)
[![Roadmap](https://img.shields.io/badge/Roadmap-2026-DA7F07.svg)](#roadmap)

## SeekStorm REST client (Rust)
[![Crates.io](https://img.shields.io/crates/v/seekstorm_client_rs.svg)](https://crates.io/crates/seekstorm_client_rs)
[![Downloads](https://img.shields.io/crates/d/seekstorm_client_rs.svg?style=flat-square)](https://crates.io/crates/seekstorm_client_rs)
[![Documentation](https://docs.rs/seekstorm_client_rs/badge.svg)](https://docs.rs/seekstorm_client_rs)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/SeekStorm/SeekStorm?tab=Apache-2.0-1-ov-file#readme)
[![Roadmap](https://img.shields.io/badge/Roadmap-2026-DA7F07.svg)](#roadmap)

## SeekStorm multi-tenancy search server
[![Crates.io](https://img.shields.io/crates/v/seekstorm_server.svg)](https://crates.io/crates/seekstorm_server)
[![Downloads](https://img.shields.io/crates/d/seekstorm_server.svg?style=flat-square)](https://crates.io/crates/seekstorm_server)
[![Docker](https://img.shields.io/docker/pulls/wolfgarbe/seekstorm_server)](https://hub.docker.com/r/wolfgarbe/seekstorm_server)
[![REST API Documentation](https://docs.rs/seekstorm/badge.svg)](https://seekstorm.github.io/documentation/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/SeekStorm/SeekStorm?tab=Apache-2.0-1-ov-file#readme)
[![Roadmap](https://img.shields.io/badge/Roadmap-2026-DA7F07.svg)](#roadmap)

## SeekStorm in-process search library
[![Crates.io](https://img.shields.io/crates/v/seekstorm.svg)](https://crates.io/crates/seekstorm)
[![Downloads](https://img.shields.io/crates/d/seekstorm.svg?style=flat-square)](https://crates.io/crates/seekstorm)
[![Documentation](https://docs.rs/seekstorm/badge.svg)](https://docs.rs/seekstorm)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/SeekStorm/SeekStorm?tab=Apache-2.0-1-ov-file#readme)
[![Roadmap](https://img.shields.io/badge/Roadmap-2026-DA7F07.svg)](#roadmap)
<p>
  <a href="https://seekstorm.com">Website</a> | 
  <a href="https://seekstorm.github.io/search-benchmark-game/">Benchmark</a> | 
  <a href="https://deephn.org/">Demo</a> | 
  <a href="https://github.com/SeekStorm/SeekStorm">Repository for SeekStorm Python client </a> | 
  <a href="https://github.com/SeekStorm/SeekStorm">Repository for SeekStorm library, server, Rust client </a> | 
  <a href="https://github.com/SeekStorm/SeekStorm#roadmap">Roadmap</a> | 
  <a href="https://seekstorm.com/blog/">Blog</a> | 
  <a href="https://x.com/seekstorm">X</a>
</p>


## Usage of the Python client

```python ,no_run
import json

from seekstorm_client_py import (
    ApikeyQuotaObject,
    CreateIndexRequest,
    GetDocumentRequest,
    PySeekStormClient as SeekStormClient,
    SearchRequestObject,
)

BASE_URL = "http://127.0.0.1:80"
DEMO_API_KEY = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
MASTER_API_KEY = "/iWStCpyfpd/BVlHOFtwnMgrFrmof4jGq/OQDWXQzcM="

client = SeekStormClient()

# Server live check

result = client.live(BASE_URL)

# Create API key

quota = ApikeyQuotaObject()
quota.indices_max = 10
quota.indices_size_max = 100_000_000_000
quota.documents_max = 100_000_000
quota.operations_max = 1_000_000_000
quota.rate_limit = None
quota.demo = True

result = client.create_apikey(BASE_URL, MASTER_API_KEY, quota)

# Create index

schema_json = """
[{"field":"title","field_type":"Text","store":false,"index_lexical":false},
{"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
{"field":"url","field_type":"Text","store":false,"index_lexical":false}]
"""

create_request = CreateIndexRequest()
create_request.index_name = "test_index"
create_request.similarity = "Bm25f"
create_request.tokenizer = "UnicodeAlphanumeric"
create_request.stemmer = "None"
create_request.document_compression = "Snappy"
create_request.schema = schema_json
create_request.ngram_indexing = 0

index_id = client.create_index(BASE_URL, DEMO_API_KEY, create_request)

# Index document

client.index_document(BASE_URL, DEMO_API_KEY, index_id, json.dumps({"title": "title2", "body": "body2 test", "url": "url2"}))

# Commit index

client.commit_index(BASE_URL, DEMO_API_KEY, index_id)

# Search index

request = SearchRequestObject("+body2 +test")
request.offset = 0
request.length = 10
request.enable_empty_query = False
request.realtime = False

result_object = client.query_index(BASE_URL, DEMO_API_KEY, index_id, request)
```


## Build seekstorm_client_py

```shell
pip install uv
python -m venv .venv
source .venv/bin/activate  # On Windows use: .venv\Scripts\activate
uv tool install maturin
maturin develop
```

## Test

Make sure you start the SeekStorm server before running these tests!

```shell
python -m unittest -v test_client.py
```