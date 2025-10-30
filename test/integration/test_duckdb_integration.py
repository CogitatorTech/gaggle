#!/usr/bin/env python3
import io
import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import ThreadingMixIn


class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True


class KaggleMockHandler(BaseHTTPRequestHandler):
    zip_bytes: bytes = b""

    def do_GET(self):
        if self.path.startswith('/datasets/list'):
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(b'[]')
            return
        if self.path == '/datasets/view/owner/dataset':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({"ref": "owner/dataset"}).encode())
            return
        if self.path == '/datasets/download/owner/dataset':
            self.send_response(200)
            self.send_header('Content-Type', 'application/zip')
            self.end_headers()
            self.wfile.write(self.zip_bytes)
            return
        self.send_response(404)
        self.end_headers()

    def log_message(self, fmt, *args):
        # Silence console spam during tests
        return


class KaggleMockServer:
    def __init__(self):
        self._server: ThreadedHTTPServer | None = None
        self._thread: threading.Thread | None = None
        self.port: int | None = None

    def start(self, zip_bytes: bytes):
        KaggleMockHandler.zip_bytes = zip_bytes
        # Bind to random port
        sock = socket.socket()
        sock.bind(("127.0.0.1", 0))
        addr, port = sock.getsockname()
        sock.close()
        self.port = port
        self._server = ThreadedHTTPServer(("127.0.0.1", port), KaggleMockHandler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    def stop(self):
        if self._server:
            self._server.shutdown()
        if self._thread:
            self._thread.join(timeout=2)


def build_test_zip() -> bytes:
    import zipfile
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr('test.csv', 'id,name\n1,Alice\n2,Bob\n')
    return buf.getvalue()


def run_duckdb(sql: str, env: dict) -> subprocess.CompletedProcess:
    duck = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'build', 'release', 'duckdb'))
    if not os.path.exists(duck):
        raise RuntimeError(f"DuckDB binary not found at {duck}; build it with 'make release'")
    return subprocess.run([duck, '-batch'], input=sql.encode('utf-8'), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          env=env, check=False)


def main():
    # Prepare mock server
    zip_bytes = build_test_zip()
    server = KaggleMockServer()
    server.start(zip_bytes)

    temp_cache = tempfile.TemporaryDirectory()
    env = os.environ.copy()
    env['KAGGLE_USERNAME'] = 'user'
    env['KAGGLE_KEY'] = 'key'
    env['GAGGLE_API_BASE'] = f'http://127.0.0.1:{server.port}'
    env['GAGGLE_CACHE_DIR'] = temp_cache.name
    env['GAGGLE_HTTP_TIMEOUT'] = '5'

    try:
        # Roundtrip SQL using the extension
        sql = """
.mode csv
.headers off
SELECT gaggle_set_credentials('user','key');
SELECT gaggle_download('owner/dataset');
SELECT COUNT(*) FROM read_csv_auto((SELECT gaggle_download('owner/dataset') || '/test.csv'));
SELECT COUNT(*) FROM 'kaggle:owner/dataset/test.csv';
SELECT COUNT(*) FROM 'kaggle:owner/dataset/*.csv';
SELECT gaggle_search('x', 1, 1);
"""
        proc = run_duckdb(sql, env)
        out = proc.stdout.decode('utf-8')
        err = proc.stderr.decode('utf-8')
        if proc.returncode != 0:
            print('DuckDB failed:', err)
            sys.exit(1)
        # Expect count '2' appears at least once (for explicit path and wildcard)
        assert out.count('\n2\n') + out.count(',2\n') >= 1, f"Expected count 2 in output, got: {out}"
        # Expect empty array [] for search
        assert '[]' in out, f"Expected [] in output for search, got: {out}"

        # Error case: missing file via replacement scan should error on query
        sql_err = """
                  SELECT COUNT(*)
                  FROM 'kaggle:owner/dataset/missing.csv'; \
                  """
        proc_err = run_duckdb(sql_err, env)
        assert proc_err.returncode != 0, "Expected failure for missing file replacement scan"

        # Error case: invalid dataset path should error on download
        sql_bad = """
SELECT gaggle_download('owner_only');
"""
        proc_bad = run_duckdb(sql_bad, env)
        assert proc_bad.returncode != 0, "Expected failure for invalid dataset path"

        print('Integration test passed')
        return 0
    finally:
        server.stop()
        temp_cache.cleanup()


if __name__ == '__main__':
    sys.exit(main())
