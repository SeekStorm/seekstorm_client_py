import json, urllib.request
base = 'http://127.0.0.1:80'
master = '/iWStCpyfpd/BVlHOFtwnMgrFrmof4jGq/OQDWXQzcM='
quota = json.dumps({'indices_max': 10, 'indices_size_max': 100000000000, 'documents_max': 100000000, 'operations_max': 1000000000, 'rate_limit': None, 'demo': True}).encode()
req = urllib.request.Request(base + '/api/v1/apikey', data=quota, headers={'apikey': master, 'Content-Type': 'application/json'}, method='POST')
with urllib.request.urlopen(req, timeout=10) as resp:
    demo = resp.read().decode().strip()
    print('demo', demo)
    idx_req = json.dumps({'index_name': 'test_index', 'similarity': 'Bm25f', 'tokenizer': 'UnicodeAlphanumeric', 'stemmer': 'None', 'document_compression': 'Snappy', 'schema': [{'field': 'title', 'field_type': 'Text', 'store': False, 'index_lexical': False}, {'field': 'body', 'field_type': 'Text', 'store': True, 'index_lexical': True, 'longest': True}, {'field': 'url', 'field_type': 'Text', 'store': False, 'index_lexical': False}], 'ngram_indexing': 0}).encode()
    idx = urllib.request.Request(base + '/api/v1/index', data=idx_req, headers={'apikey': demo, 'Content-Type': 'application/json'}, method='POST')
    with urllib.request.urlopen(idx, timeout=10) as resp2:
        print('index_status', resp2.status)
        print('index_body', resp2.read().decode())
    doc_req = json.dumps({'title': 'title1 test', 'body': 'body1', 'url': 'url1'}).encode()
    doc = urllib.request.Request(base + '/api/v1/index/0/doc', data=doc_req, headers={'apikey': demo, 'Content-Type': 'application/json'}, method='POST')
    try:
        with urllib.request.urlopen(doc, timeout=10) as resp3:
            print('doc_status', resp3.status)
            print('doc_body', resp3.read().decode())
    except Exception as e:
        print(type(e).__name__, e)
        if hasattr(e, 'read'):
            print(e.read().decode())
