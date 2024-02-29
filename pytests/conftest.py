import json
import os
from typing import Any, List, Tuple


def load_data_fixtures(dir_path: str) -> List[Tuple[str, Any]]:
    data = []
    for file in os.listdir(dir_path):
        if not file.endswith('.json'):
            continue

        with open(os.path.join(dir_path, file), 'rb') as f:
            data.append((file, json.load(f)))

    return data
