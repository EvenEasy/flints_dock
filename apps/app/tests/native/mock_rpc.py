#!/usr/bin/env python3
"""Local deterministic Solana fixture for native Tauri smoke tests; never forwards RPC."""
import argparse
import base64
import json
import struct
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
TOKEN = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'


def b58(value):
    number = int.from_bytes(value, 'big')
    result = ''
    while number:
        number, digit = divmod(number, 58)
        result = ALPHABET[digit] + result
    return '1' * (len(value) - len(value.lstrip(b'\0'))) + result


# This public fixture identity is derived from a published all-zero test seed.
OWNER_BYTES = bytes.fromhex('3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29')
OWNER = b58(OWNER_BYTES)
MINT = b58(bytes([3]) * 32)
ACCOUNTS = [b58(bytes([4]) * 32), b58(bytes([5]) * 32)]
closed = set()
signatures = {}
lock = threading.Lock()


def account(data, lamports):
    return {'lamports': lamports, 'owner': TOKEN, 'data': [base64.b64encode(data).decode(), 'base64'],
            'executable': False, 'rentEpoch': 0}


token_bytes = bytearray(165)
token_bytes[:32] = bytes([3]) * 32
token_bytes[32:64] = OWNER_BYTES
token_bytes[108] = 1
TOKEN_ACCOUNT = account(token_bytes, 2039280)
mint_bytes = bytearray(82)
mint_bytes[36:44] = struct.pack('<Q', 100000000)
mint_bytes[44:46] = bytes([6, 1])
MINT_ACCOUNT = account(mint_bytes, 1000000)


def close_transaction(encoded):
    """Decode only the fixture's shortvec/legacy-or-v0 transaction and enforce close destinations."""
    data = base64.b64decode(encoded)
    offset = 0

    def short():
        nonlocal offset
        value, shift = 0, 0
        while True:
            byte = data[offset]
            offset += 1
            value |= (byte & 127) << shift
            if not byte & 128:
                return value
            shift += 7
            assert shift <= 21

    count = short()
    assert count == 1
    signature = b58(data[offset:offset + 64])
    offset += 64
    if data[offset] & 128:
        offset += 1
    offset += 3
    keys = []
    for _ in range(short()):
        keys.append(b58(data[offset:offset + 32]))
        offset += 32
    offset += 32
    source = None
    for _ in range(short()):
        program = data[offset]
        offset += 1
        length = short()
        indices = data[offset:offset + length]
        offset += length
        length = short()
        instruction = data[offset:offset + length]
        offset += length
        if instruction == bytes([9]) and keys[program] == TOKEN:
            assert keys[indices[1]] == OWNER == keys[indices[2]]
            source = keys[indices[0]]
    assert source in ACCOUNTS and source not in closed
    closed.add(source)
    signatures[signature] = source
    return signature


def rpc(method, params):
    if method == 'getGenesisHash':
        return 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG'
    if method == 'getTokenAccountsByOwner':
        assert params[0] == OWNER
        rows = [] if params[1]['programId'] != TOKEN else [
            {'pubkey': source, 'account': TOKEN_ACCOUNT} for source in ACCOUNTS if source not in closed]
        return {'context': {'slot': 1}, 'value': rows}
    if method == 'getMultipleAccounts':
        return {'context': {'slot': 1}, 'value': [MINT_ACCOUNT if key == MINT else None for key in params[0]]}
    if method == 'getAccountInfo':
        return {'context': {'slot': 1}, 'value': TOKEN_ACCOUNT if params[0] in ACCOUNTS and params[0] not in closed else None}
    if method == 'getProgramAccounts':
        return []
    if method == 'getBalance':
        return {'context': {'slot': 1}, 'value': 9007199254740993123}
    if method == 'getLatestBlockhash':
        return {'context': {'slot': 1}, 'value': {'blockhash': '11111111111111111111111111111111', 'lastValidBlockHeight': 1000}}
    if method == 'getBlockHeight':
        return 10
    if method == 'getVersion':
        return {'solana-core': '3.0.0', 'feature-set': 0}
    if method == 'simulateTransaction':
        assert params[1]['sigVerify'] is True
        return {'context': {'slot': 1}, 'value': {'err': None, 'logs': [], 'unitsConsumed': 3000}}
    if method == 'sendTransaction':
        assert params[1]['skipPreflight'] is False
        return close_transaction(params[0])
    if method == 'getSignatureStatuses':
        time.sleep(confirmation_delay)
        return {'context': {'slot': 1}, 'value': [
            {'slot': 1, 'confirmations': None, 'err': None, 'status': {'Ok': None}, 'confirmationStatus': 'confirmed'}
            if signature in signatures else None for signature in params[0]]}
    if method == 'getTransaction':
        return {'transaction': {'message': {'accountKeys': [OWNER, signatures[params[0]]]}},
                'meta': {'err': None, 'preBalances': [9007199254740993123, 2039280],
                         'postBalances': [9007199254743027403, 0]}}
    raise ValueError('Unexpected fixture RPC: ' + method)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        payload = {'owner': OWNER, 'mint': MINT, 'closed': sorted(closed), 'signatures': list(signatures)}
        self.reply(payload)

    def reply(self, payload):
        body = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        try:
            with lock:
                result = rpc(request['method'], request.get('params', []))
            payload = {'jsonrpc': '2.0', 'id': request['id'], 'result': result}
        except Exception as error:
            payload = {'jsonrpc': '2.0', 'id': request['id'], 'error': {'code': -32000, 'message': str(error)}}
        self.reply(payload)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=18999)
    parser.add_argument('--confirmation-delay', type=float, default=0, help='Delay mock RPC replies for observing real waiting stages')
    args = parser.parse_args()
    confirmation_delay = max(0, args.confirmation_delay)
    print(json.dumps({'url': f'http://127.0.0.1:{args.port}', 'owner': OWNER, 'mint': MINT}), flush=True)
    ThreadingHTTPServer(('127.0.0.1', args.port), Handler).serve_forever()
