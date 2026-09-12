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

    Raises:
        ValueError: If the input is not a valid CID or is of an unsupported type.
    """


def encode_cid(data: str | bytes) -> str:
    """Encode a CID to its string representation.

    Args:
        data: Either a CID string (will be returned as-is) or raw CID bytes

    Returns:
        A CID string (e.g. 'bafy...')

    Raises:
        ValueError: If the input is not a valid CID or is of an unsupported type.
    """


def decode_car(data: bytes) -> tuple[dict[str, Any], dict[bytes, dict[str, Any]]]:
    """Decode a CAR file.

    Args:
        data: Raw CAR file bytes

    Returns:
        A tuple containing:
        - header: dict (CAR header)
        - blocks: dict mapping CID bytes to block data

    Raises:
        ValueError: If the header or any block is malformed. This includes
            a non-DAG-CBOR block codec and text strings that are not valid UTF-8.
        RecursionError: If a block is nested deeper than sys.getrecursionlimit().
    """


def decode_dag_cbor(data: bytes) -> Any:
    """Decode DAG-CBOR data to Python objects.

    Args:
        data: Raw DAG-CBOR bytes

    Returns:
        A Python object

    Raises:
        ValueError: If the data is malformed or is not strict DAG-CBOR. This
            includes trailing data, unsupported tags, non-string map keys, and
            text strings that are not valid UTF-8.
        RecursionError: If the data is nested deeper than sys.getrecursionlimit().
    """


def decode_dag_cbor_multi(data: bytes) -> list[Any]:
    """Decode multiple DAG-CBOR objects from bytes.

    Args:
        data: Raw DAG-CBOR bytes containing multiple objects

    Returns:
        A list of Python objects

    Raises:
        ValueError: If any object is malformed or is not strict DAG-CBOR. This
            includes unsupported tags, non-string map keys, and text strings
            that are not valid UTF-8.
        RecursionError: If an object is nested deeper than sys.getrecursionlimit().
    """


def encode_dag_cbor(data: Any) -> bytes:
    """Encode Python objects to DAG-CBOR.

    Args:
        data: Any Python object that can be encoded to DAG-CBOR

    Returns:
        Raw DAG-CBOR bytes

    Raises:
        ValueError: If the object cannot be represented in DAG-CBOR. This
            includes unsupported types, integers outside the 64-bit range,
            non-finite floats, and map keys that are not strings.
    """


def decode_multibase(data: str) -> tuple[str, bytes]:
    """Decode multibase-encoded data.

    Args:
        data: Multibase-encoded string (e.g. 'ueWVzIG1hbmkgIQ')

    Returns:
        A tuple containing:
        - base: str (the base code, e.g. 'u')
        - data: bytes (the decoded data)

    Raises:
        ValueError: If the input is not valid multibase.
    """


def encode_multibase(code: str, data: str | bytes) -> str:
    """Encode data using multibase.

    Args:
        code: Base code (e.g. 'u' for base58btc)
        data: Data to encode (bytes or string that can be converted to bytes)

    Returns:
        Multibase-encoded string

    Raises:
        ValueError: If the base code is unknown or the data is of an unsupported type.
    """
