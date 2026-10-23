#!/usr/bin/env python3
"""Local HTTP/preflight smoke using only the security gate's disposable services."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit

root = Path(__file__).resolve().parent.parent
url = os.environ['TEST_DATABASE_URL']
assert urlsplit(url).hostname == '127.0.0.1'
assert urlsplit(url).path == '/astra_isolation_test'
redis = os.environ['TEST_REDIS_URL']
assert urlsplit(redis).hostname == '127.0.0.1'
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
base = f'http://127.0.0.1:{port}'
origin = 'http://localhost:3000'
env = {key: value for key, value in os.environ.items() if not key.startswith(('SUPABASE_', 'ALCHEMY_', 'MORALIS_'))}
env.update(DATABASE_URL=url, REDIS_URL=redis, NODE_ENV='development', ALLOW_DEV_AUTH='true',
           DEV_USER_ID='isolation-local-a', HOST='127.0.0.1', PORT=str(port), WEB_ORIGIN=origin,
           SUPABASE_URL='', SUPABASE_SERVICE_ROLE_KEY='', ALCHEMY_API_KEY='', MORALIS_API_KEY='')
target = Path(os.environ.get('CARGO_TARGET_DIR', root / 'apps/api-rust/target'))
if not target.is_absolute():
    target = root / 'apps/api-rust' / target

def request(path, method='GET', body=None, headers=None):
    req = urllib.request.Request(base + path, data=None if body is None else json.dumps(body).encode(),
                                 method=method, headers=headers or {})
    try:
        response = urllib.request.urlopen(req, timeout=10)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        payload = response.read()
        return response.status, response.headers, json.loads(payload) if payload else None

with tempfile.TemporaryDirectory(prefix='astra-http-') as directory, tempfile.TemporaryFile() as logs:
    process = subprocess.Popen([str(target / 'debug/astra-api')], cwd=directory, env=env, stdout=logs, stderr=logs)
    try:
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError('Rust API exited before readiness')
            try:
                status, _, health = request('/v1/health')
                if status == 200 and health['status'] == 'ok':
                    break
            except (OSError, urllib.error.URLError):
                pass
            time.sleep(0.1)
        else:
            raise RuntimeError('Rust API did not become healthy')
        status, headers, _ = request('/v1/me/portfolio/snapshots-usd', 'OPTIONS', headers={
            'Origin': origin, 'Access-Control-Request-Method': 'POST',
            'Access-Control-Request-Headers': 'authorization,content-type'})
        assert status == 200
        assert headers['Access-Control-Allow-Origin'] == origin
        assert 'POST' in headers['Access-Control-Allow-Methods']
        assert 'authorization' in headers['Access-Control-Allow-Headers']
        assert headers['Access-Control-Allow-Credentials'] == 'true'
        _, headers, _ = request('/v1/me/wallets', 'OPTIONS', headers={
            'Origin': 'http://untrusted.invalid', 'Access-Control-Request-Method': 'DELETE'})
        assert headers.get('Access-Control-Allow-Origin') != 'http://untrusted.invalid'
        address = '0x0000000000000000000000000000000000000001'
        status, _, summary = request('/v1/wallet/' + address, headers={'Origin': origin})
        assert status == 200 and summary['address'] == address
        assert summary['alchemy'] is None and summary['moralis'] is None
        assert request('/v1/wallet/invalid')[0] == 400
        route = '/v1/me/portfolio/snapshots-usd'
        status, headers, snapshot = request(route, 'POST', {'totalUsd': 42.125, 'userId': 'isolation-local-b'}, {'Content-Type': 'application/json'})
        assert status == 201 and snapshot['totalUsd'] == '42.13'
        assert snapshot['userId'] == 'isolation-local-a'
        assert headers['Cache-Control'] == 'private, no-store'
        assert any(row['id'] == snapshot['id'] for row in request(route)[2])
        assert all(row['id'] != snapshot['id'] for row in request(route, headers={'X-User-Id': 'isolation-local-b'})[2])
        assert request(route, headers={'Authorization': 'Bearer invalid'})[0] == 401
        print('Rust HTTP smoke passed: health, public wallet, USD owner separation and CORS/preflight (synthetic local data).')
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
