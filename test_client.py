# Make sure you start the SeekStorm server before running these tests!
# maturin develop
# python -m unittest -v test_client.py

import json
import unittest

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


class TestSeekStormClient(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.client = SeekStormClient()

    def _normalize_live_response(self, response):
        if not isinstance(response, str):
            return str(response)

        try:
            parsed = json.loads(response)
        except (TypeError, json.JSONDecodeError):
            return response

        return parsed if isinstance(parsed, str) else str(parsed)

    def setUp(self):
        status = self._normalize_live_response(self.client.live(BASE_URL))
        self.assertTrue(
            "SeekStorm" in status,
            f"SeekStorm server is not reachable at {BASE_URL}: {status}",
        )

    def test_20_live(self):
        result = self._normalize_live_response(self.client.live(BASE_URL))
        self.assertIsInstance(result, str)
        self.assertIn("SeekStorm", result)

    def test_21_create_apikey(self):
        quota = ApikeyQuotaObject()
        quota.indices_max = 10
        quota.indices_size_max = 100_000_000_000
        quota.documents_max = 100_000_000
        quota.operations_max = 1_000_000_000
        quota.rate_limit = None
        quota.demo = True

        result = self.client.create_apikey(BASE_URL, MASTER_API_KEY, quota)
        self.assertEqual(result, DEMO_API_KEY)

    def test_22_create_index(self):
        schema_json = """
        [{"field":"title","field_type":"Text","store":false,"index_lexical":false},
        {"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
        {"field":"url","field_type":"Text","store":false,"index_lexical":false}]
        """

        request = CreateIndexRequest()
        request.index_name = f"test_index_{self._testMethodName}"
        request.similarity = "Bm25f"
        request.tokenizer = "UnicodeAlphanumeric"
        request.stemmer = "None"
        request.document_compression = "Snappy"
        request.schema = schema_json
        request.ngram_indexing = 0

        result = self.client.create_index(BASE_URL, DEMO_API_KEY, request)
        self.assertGreaterEqual(result, 0)

    def test_23_index_documents(self):
        schema_json = """
        [{"field":"title","field_type":"Text","store":false,"index_lexical":false},
        {"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
        {"field":"url","field_type":"Text","store":false,"index_lexical":false}]
        """

        create_request = CreateIndexRequest()
        create_request.index_name = f"test_index_{self._testMethodName}"
        create_request.similarity = "Bm25f"
        create_request.tokenizer = "UnicodeAlphanumeric"
        create_request.stemmer = "None"
        create_request.document_compression = "Snappy"
        create_request.schema = schema_json
        create_request.ngram_indexing = 0
        index_id = self.client.create_index(BASE_URL, DEMO_API_KEY, create_request)

        document_json = json.dumps({"title": "title1 test", "body": "body1", "url": "url1"})
        self.client.index_document(BASE_URL, DEMO_API_KEY, index_id, document_json)

        documents_json = json.dumps(
            [
                {"title": "title1 test", "body": "body1", "url": "url1"},
                {"title": "title2", "body": "body2 test", "url": "url2"},
                {"title": "title3 test", "body": "body3 test", "url": "url3"},
            ]
        )
        self.client.index_documents(BASE_URL, DEMO_API_KEY, index_id, documents_json)

        result = self.client.commit_index(BASE_URL, DEMO_API_KEY, index_id)
        self.assertEqual(result, 4)

    def test_24_query_index(self):
        schema_json = """
        [{"field":"title","field_type":"Text","store":false,"index_lexical":false},
        {"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
        {"field":"url","field_type":"Text","store":false,"index_lexical":false}]
        """

        create_request = CreateIndexRequest()
        create_request.index_name = f"test_index_{self._testMethodName}"
        create_request.similarity = "Bm25f"
        create_request.tokenizer = "UnicodeAlphanumeric"
        create_request.stemmer = "None"
        create_request.document_compression = "Snappy"
        create_request.schema = schema_json
        create_request.ngram_indexing = 0
        index_id = self.client.create_index(BASE_URL, DEMO_API_KEY, create_request)

        self.client.index_document(BASE_URL, DEMO_API_KEY, index_id, json.dumps({"title": "title2", "body": "body2 test", "url": "url2"}))
        self.client.commit_index(BASE_URL, DEMO_API_KEY, index_id)

        request = SearchRequestObject("+body2 +test")
        request.offset = 0
        request.length = 10
        request.enable_empty_query = False
        request.realtime = False

        result_object = self.client.query_index(BASE_URL, DEMO_API_KEY, index_id, request)
        self.assertEqual(result_object.count_total, 1)

    def test_25_get_document(self):
        schema_json = """
        [{"field":"title","field_type":"Text","store":false,"index_lexical":false},
        {"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
        {"field":"url","field_type":"Text","store":false,"index_lexical":false}]
        """

        create_request = CreateIndexRequest()
        create_request.index_name = f"test_index_{self._testMethodName}"
        create_request.similarity = "Bm25f"
        create_request.tokenizer = "UnicodeAlphanumeric"
        create_request.stemmer = "None"
        create_request.document_compression = "Snappy"
        create_request.schema = schema_json
        create_request.ngram_indexing = 0
        index_id = self.client.create_index(BASE_URL, DEMO_API_KEY, create_request)

        self.client.index_document(BASE_URL, DEMO_API_KEY, index_id, json.dumps({"title": "title3", "body": "body3 test", "url": "url3"}))
        self.client.commit_index(BASE_URL, DEMO_API_KEY, index_id)

        request = GetDocumentRequest()
        request.query_terms = []
        request.fields = []

        raw_response = self.client.get_document(BASE_URL, DEMO_API_KEY, index_id, 0, request)
        documents = json.loads(raw_response)
        self.assertEqual(len(documents), 1)

    def test_26_clear_index(self):
        schema_json = """
        [{"field":"title","field_type":"Text","store":false,"index_lexical":false},
        {"field":"body","field_type":"Text","store":true,"index_lexical":true,"longest":true},
        {"field":"url","field_type":"Text","store":false,"index_lexical":false}]
        """

        create_request = CreateIndexRequest()
        create_request.index_name = f"test_index_{self._testMethodName}"
        create_request.similarity = "Bm25f"
        create_request.tokenizer = "UnicodeAlphanumeric"
        create_request.stemmer = "None"
        create_request.document_compression = "Snappy"
        create_request.schema = schema_json
        create_request.ngram_indexing = 0
        index_id = self.client.create_index(BASE_URL, DEMO_API_KEY, create_request)

        self.client.index_document(BASE_URL, DEMO_API_KEY, index_id, json.dumps({"title": "title3", "body": "body3 test", "url": "url3"}))
        self.client.commit_index(BASE_URL, DEMO_API_KEY, index_id)

        request = SearchRequestObject("+body3 +test")
        request.offset = 0
        request.length = 10
        request.enable_empty_query = False
        request.realtime = False

        before_clear = self.client.query_index(BASE_URL, DEMO_API_KEY, index_id, request)
        self.assertEqual(before_clear.count_total, 1)

        raw_response = self.client.clear_index(BASE_URL, DEMO_API_KEY, index_id)

        self.assertEqual(raw_response, 0)

        after_clear = self.client.query_index(BASE_URL, DEMO_API_KEY, index_id, request)
        self.assertEqual(after_clear.count_total, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
