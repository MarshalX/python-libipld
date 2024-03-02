import json
import os
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
