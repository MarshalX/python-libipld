from __future__ import annotations
from typing import Any

def decode_cid(data: str | bytes) -> dict[str, Any]:
    """Decode a CID from either its string representation or raw bytes.

    Args:
        data: Either a CID string (e.g. 'bafy...') or raw CID bytes

    Returns:
        A dict containing:
        - version: int (0 or 1)
        - codec: int (e.g. 113 for DAG-CBOR)
        - hash: dict containing:
            - code: int (hash algorithm code)
            - size: int (hash size in bytes)
            - digest: bytes (hash digest)
    """

def encode_cid(data: str | bytes) -> str:
    """Encode a CID to its string representation.

    Args:
        data: Either a CID string (will be returned as-is) or raw CID bytes

    Returns:
        A CID string (e.g. 'bafy...')
    """

def decode_car(data: bytes) -> tuple[dict[str, Any], dict[bytes, dict[str, Any]]]:
    """Decode a CAR file.

    Args:
        data: Raw CAR file bytes

    Returns:
        A tuple containing:
        - header: dict (CAR header)
        - blocks: dict mapping CID bytes to block data
    """

def decode_dag_cbor(data: bytes) -> Any:
    """Decode DAG-CBOR data to Python objects.

    Args:
        data: Raw DAG-CBOR bytes

    Returns:
        A Python object (dict, list, str, bytes, int, float, bool, or None)
    """

def decode_dag_cbor_multi(data: bytes) -> list[Any]:
    """Decode multiple DAG-CBOR objects from bytes.

    Args:
        data: Raw DAG-CBOR bytes containing multiple objects

    Returns:
        A list of Python objects
    """

def encode_dag_cbor(data: Any) -> bytes:
    """Encode Python objects to DAG-CBOR.

    Args:
        data: Any Python object that can be encoded to DAG-CBOR
              (dict, list, str, bytes, int, float, bool, or None)

    Returns:
        Raw DAG-CBOR bytes
    """

def decode_multibase(data: str) -> tuple[str, bytes]:
    """Decode multibase-encoded data.

    Args:
        data: Multibase-encoded string (e.g. 'u' for base58btc)

    Returns:
        A tuple containing:
        - base: str (the base code, e.g. 'u')
        - data: bytes (the decoded data)
    """

def encode_multibase(code: str, data: Any) -> str:
    """Encode data using multibase.

    Args:
        code: str (base code, e.g. 'u' for base58btc)
        data: Any Python object that can be converted to bytes
              (str, bytes, bytearray, or other objects that can be converted to bytes)

    Returns:
        Multibase-encoded string
    """
