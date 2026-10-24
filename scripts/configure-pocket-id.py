#!/usr/bin/env python3
"""Provision household-restricted Recetaco public PKCE clients and API grants."""
import json
from pathlib import Path
import urllib.request
import urllib.error

BASE = 'https://auth.aries.timdumol.com'
ORIGIN = 'https://recetaco.timdumol.com'
SUBJECTS = ['6aca99a5-7ec4-4252-b94e-a0e53805e983', 'adaec307-5a7b-4c16-b854-2fe3752e6ea3']
KEY = (Path(__file__).resolve().parent.parent / '.pocket-id-api-key').read_text().strip()

def request(method, path, body=None):
    req = urllib.request.Request(BASE + path,
        data=None if body is None else json.dumps(body).encode(), method=method,
        headers={'X-API-KEY': KEY, 'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404 and method == 'GET':
            return None
        raise RuntimeError(f'Pocket ID {method} {path}: HTTP {error.code}') from None

# Provision and verify exact household membership before granting any client access.
groups = request('GET', '/api/user-groups?pagination%5Blimit%5D=100')['data']
group = next((g for g in groups if g['name'] == 'recetaco-household'), None)
if group is None:
    group = request('POST', '/api/user-groups',
        {'name': 'recetaco-household', 'friendlyName': 'Recetaco household'})
request('PUT', '/api/user-groups/' + group['id'] + '/users', {'userIds': SUBJECTS})
verified_group = request('GET', '/api/user-groups/' + group['id'])
assert sorted(u['id'] for u in verified_group['users']) == sorted(SUBJECTS)

apis = request('GET', '/api/apis?pagination%5Blimit%5D=100')['data']
api = next((a for a in apis if a['resource'] == ORIGIN), None)
if api is None:
    api = request('POST', '/api/apis', {'name': 'Recetaco', 'resource': ORIGIN})
permissions = api.get('permissions', [])
if not any(p['key'] == 'recetaco:access' for p in permissions):
    api = request('PUT', '/api/apis/' + api['id'] + '/permissions', {
        'permissions': [{k: p.get(k, '') for k in ('key', 'name', 'description')} for p in permissions]
        + [{'key': 'recetaco:access', 'name': 'Shared kitchen access', 'description': 'Use the shared Recetaco kitchen'}]})
permission = next(p['id'] for p in api['permissions'] if p['key'] == 'recetaco:access')
for client_id, name, callback, logout in [
    ('recetaco-web', 'Recetaco Browser', ORIGIN + '/auth/callback', [ORIGIN]),
    ('recetaco-android', 'Recetaco Android', 'dev.recetaco.app:/oauth2redirect', []),
]:
    existing = request('GET', '/api/oidc/clients/' + client_id)
    payload = dict(name=name, description='Recetaco shared household kitchen',
        callbackURLs=[callback], logoutCallbackURLs=logout, isPublic=True,
        pkceEnabled=True, requiresReauthentication=False,
        requiresPushedAuthorizationRequests=False, skipConsent=False,
        credentials={}, launchURL=ORIGIN if client_id == 'recetaco-web' else None,
        isGroupRestricted=True)
    if existing is None:
        request('POST', '/api/oidc/clients', dict(payload, id=client_id))
    else:
        request('PUT', '/api/oidc/clients/' + client_id, payload)
    request('PUT', '/api/oidc/clients/' + client_id + '/allowed-user-groups',
        {'userGroupIds': [group['id']]})
    verified = request('GET', '/api/oidc/clients/' + client_id)
    assert verified['isPublic'] and verified['pkceEnabled'] and verified['isGroupRestricted']
    assert [g['id'] for g in verified['allowedUserGroups']] == [group['id']]
    assert verified['callbackURLs'] == [callback]
    # Only verified household-restricted clients receive delegated kitchen access.
    request('PUT', '/api/api-access/' + client_id,
        {'userDelegatedPermissionIds': [permission], 'clientPermissionIds': []})
    grants = request('GET', '/api/api-access/' + client_id)
    assert grants['userDelegatedPermissionIds'] == [permission]
    assert grants['clientPermissionIds'] == []
    print('Verified household-restricted PKCE client:', client_id)
print('Verified API audience:', ORIGIN)
