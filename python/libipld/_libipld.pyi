from typing import Any, final

def decode_cid(data: str | bytes | CID) -> dict[str, Any]:
    """Decode a CID from either its string representation or raw bytes.

    Args:
        data: Either a CID string (e.g. 'bafy...'), raw CID bytes or a CID

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

def encode_cid(data: str | bytes | CID) -> str:
    """Encode a CID to its string representation.

    Args:
        data: Either a CID string (will be returned as-is), raw CID bytes or a CID

    Returns:
        A CID string (e.g. 'bafy...')

    Raises:
        ValueError: If the input is not a valid CID or is of an unsupported type.
    """

@final
class Multihash:
    """The multihash of a CID, from CID.hash; it cannot be constructed directly. Immutable and equal by value."""

    @property
    def code(self) -> int: ...
    @property
    def size(self) -> int: ...
    @property
    def digest(self) -> bytes: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

@final
class CID:
    """A CID (Content IDentifier). DAG-CBOR links decode to CID, and CID encodes as a link.

    Immutable and hashable. Equal only to a CID with the same bytes, never to a str or bytes.
    Supports pickle and copy; not orderable.
    """

    def __new__(cls, data: str | bytes | bytearray | CID) -> CID:
        """Parse a CID from its string form or its bytes. Returns data itself if it is a CID.

        Args:
            data: A CIDv1 in any multibase (e.g. 'bafy...'), a CIDv0 in base58btc
                without a prefix (e.g. 'Qm...'), or the binary CID. Paths such as
                '/ipfs/...' and a CIDv0 with a multibase prefix are rejected.

        Raises:
            ValueError: If data is not exactly one valid CID.
            TypeError: If data is of another type.
        """

    @classmethod
    def decode(cls, data: str | bytes | bytearray | CID) -> CID:
        """Same as CID(data)."""

    @property
    def version(self) -> int: ...
    @property
    def codec(self) -> int: ...
    @property
    def hash(self) -> Multihash: ...
    def encode(self) -> str:
        """The string form: base32 for CIDv1, base58btc without a prefix for CIDv0. Same as str(cid)."""

    def __bytes__(self) -> bytes:
        """The binary CID."""

    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

def decode_car(data: bytes) -> tuple[dict[str, Any], dict[CID, dict[str, Any]]]:
    """Decode a CAR file.

    Args:
        data: Raw CAR file bytes

    Returns:
        A tuple containing:
        - header: dict (CAR header)
        - blocks: dict mapping the CID of each block to its data

    Raises:
        ValueError: If the header or any block is malformed. This includes
            a non-DAG-CBOR block codec and text strings that are not valid UTF-8.
        RecursionError: If a block is nested deeper than sys.getrecursionlimit().
    """

def decode_dag_cbor(data: bytes, *, canonical: bool = False) -> Any:
    """Decode DAG-CBOR data to Python objects. A link decodes to CID.

    Args:
        data: Raw DAG-CBOR bytes
        canonical: Also reject data that is not in the canonical form, the
            only encoding of its value that DAG-CBOR allows.

    Returns:
        A Python object

    Raises:
        ValueError: If the data is malformed or is not strict DAG-CBOR. This
            includes trailing data, unsupported tags, non-string map keys,
            text strings that are not valid UTF-8, and links that are not
            exactly one CID. With canonical, it also includes integers, lengths
            and tags not in their shortest form, and 32-bit floats.
        RecursionError: If the data is nested deeper than sys.getrecursionlimit().
    """

def decode_dag_cbor_multi(data: bytes, *, canonical: bool = False) -> list[Any]:
    """Decode multiple DAG-CBOR objects from bytes.

    Args:
        data: Raw DAG-CBOR bytes containing multiple objects
        canonical: Also reject objects that are not in the canonical form.

    Returns:
        A list of Python objects

    Raises:
        ValueError: If any object is malformed or is not strict DAG-CBOR, as
            in decode_dag_cbor.
        RecursionError: If an object is nested deeper than sys.getrecursionlimit().
    """

def encode_dag_cbor(data: Any) -> bytes:
    """Encode Python objects to canonical DAG-CBOR.

    A CID encodes as a link, bytes as a byte string, and a tuple (not a subclass
    such as a namedtuple) as an array.

    Args:
        data: Any Python object that can be encoded to DAG-CBOR

    Returns:
        Raw DAG-CBOR bytes

    Raises:
        ValueError: If the object cannot be represented in DAG-CBOR. This
            includes unsupported types, integers outside the 64-bit range,
            non-finite floats, and map keys that are not strings.
        RecursionError: If the object is nested deeper than sys.getrecursionlimit().
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
        code: Base code (e.g. 'u' for base64url, 'z' for base58btc)
        data: Data to encode (bytes or string that can be converted to bytes)

    Returns:
        Multibase-encoded string

    Raises:
        ValueError: If the base code is unknown or the data is of an unsupported type.
    """
