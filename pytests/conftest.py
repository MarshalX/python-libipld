import json
import os
import urllib.request
from typing import Any, List, Tuple


def load_json_data_fixtures(dir_path: str) -> List[Tuple[str, Any]]:
    data = []
    for file in os.listdir(dir_path):
        if not file.endswith('.json'):
            continue

        with open(os.path.join(dir_path, file), 'rb') as f:
            data.append((file, json.load(f)))

    return data


def load_cbor_data_fixtures(dir_path: str) -> List[Tuple[str, Any]]:
    fixtures = []
    for root, folder, files in os.walk(dir_path):
        fixture = {}
        for file in files:
            if file in ('.DS_Store',):
                continue

            file_ext = file.split('.')[-1]

            with open(os.path.join(root, file), 'rb') as f:
                fixture[file_ext] = f.read()

        if fixture:
            fixtures.append((os.path.basename(root), fixture))

    return fixtures


def load_car_fixture(_: str, path: str) -> bytes:
    if os.path.exists(path):
        with open(path, 'rb') as f:
            return f.read()

    url = 'https://github.com/MarshalX/python-libipld/releases/download/v1.0.0/test_huge_repo.car'

    # Bsky team disabled the endpoint below.
    # We could not rely on it anymore.
    # Request forbidden by administrative rules (403 Forbidden).
    # url = f'https://bsky.network/xrpc/com.atproto.sync.getRepo?did={did}'

    contents = urllib.request.urlopen(url).read()
    with open(path, 'wb') as f:
        f.write(contents)

    return contents
