import threading
import time

import libipld
import pytest

# Mutating an argument from another thread mid-call is a data race on the
# free-threaded build. A clean `ValueError` is acceptable; a crash is not.

_DURATION = 0.3
_READERS = 4


def _race(mutate, call) -> None:
    deadline = time.monotonic() + _DURATION
    errors = []

    def writer() -> None:
        while time.monotonic() < deadline:
            mutate()

    def reader() -> None:
        while time.monotonic() < deadline:
            try:
                call()
            except ValueError:
                pass
            except Exception as e:  # noqa: BLE001
                errors.append(e)

    threads = [threading.Thread(target=writer)] + [threading.Thread(target=reader) for _ in range(_READERS)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    assert not errors, errors


@pytest.mark.thread_unsafe(reason='spawns its own threads')
def test_encode_list_mutated_concurrently() -> None:
    data = {'a': [{'k': [1, 2, 3]} for _ in range(200)], 'b': list(range(2000))}

    def mutate() -> None:
        data['a'] = [{'k': [i]} for i in range(200)]
        data['a'].clear()
        data['b'].clear()
        data['b'].extend(range(2000))

    _race(mutate, lambda: libipld.encode_dag_cbor(data))


@pytest.mark.thread_unsafe(reason='spawns its own threads')
def test_encode_dict_mutated_concurrently() -> None:
    data = {str(i): 'x' * 10 for i in range(500)}

    def mutate() -> None:
        for i in range(500):
            data.pop(str(i), None)
        for i in range(500):
            data[str(i)] = 'y' * 10

    _race(mutate, lambda: libipld.encode_dag_cbor(data))


@pytest.mark.thread_unsafe(reason='spawns its own threads')
def test_bytearray_resized_concurrently() -> None:
    cid = bytes.fromhex('01711220' + '00' * 32)
    buf = bytearray(cid)

    def mutate() -> None:
        buf[:] = b''
        buf[:] = cid + b'\x00' * 100_000
        buf[:] = cid

    def call() -> None:
        libipld.decode_cid(buf)
        libipld.encode_multibase('b', buf)

    _race(mutate, call)


@pytest.mark.thread_unsafe(reason='spawns its own threads')
def test_shared_data_roundtrip_concurrently() -> None:
    obj = {'$type': 'app.bsky.feed.post', 'text': 'hi', 'n': [1, 2.5, None, True, b'x'] * 50}
    encoded = libipld.encode_dag_cbor(obj)
    mismatches = []

    def call() -> None:
        decoded = libipld.decode_dag_cbor(encoded)
        if decoded != obj or libipld.encode_dag_cbor(decoded) != encoded:
            mismatches.append(decoded)

    _race(lambda: time.sleep(0.01), call)

    assert not mismatches
