#!/usr/bin/env python3
"""Read-only checks for public HTTPS, OIDC settings, and protected API/MCP."""
import json
import urllib.request
import urllib.error

BASE = 'https://recetaco.timdumol.com'

def get(path, expected=200, data=None):
    req = urllib.request.Request(BASE + path, data=data,
        headers={'Content-Type': 'application/json', 'User-Agent': 'Recetaco-Deployment-Check/1.0'})
    try:
        response = urllib.request.urlopen(req, timeout=20)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        assert response.status == expected, (path, response.status)
        return response.headers, response.read()

_, page = get('/')
assert b'<html' in page
get('/health')
_, body = get('/auth/config')
config = json.loads(body)
assert config['mode'] == 'oidc'
assert config['oidc']['issuer'] == 'https://auth.aries.timdumol.com'
assert config['oidc']['audience'] == BASE
assert config['oidc']['client_id'] == 'recetaco-web'
assert config['oidc']['android_client_id'] == 'recetaco-android'
_, body = get('/.well-known/oauth-protected-resource')
assert json.loads(body)['resource'] == BASE
for path, data in [('/api/recipes', None), ('/mcp', b'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}')]:
    headers, _ = get(path, 401, data)
    assert BASE + '/.well-known/oauth-protected-resource' in headers['WWW-Authenticate']
print('Public HTTPS, UI, health, Pocket ID configuration, resource metadata, and API/MCP authentication checks passed.')
