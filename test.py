import os.path

import libipld


def main():
    print(libipld.decode_cid('bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa'))

    for i in range(0, 100):
        with open(os.path.join('payloads', f'{i}.txt'), 'rb') as f:
            cbor_parts = libipld.decode_dag_multi(f.read())
            assert len(cbor_parts) == 2
            header, body = cbor_parts
            if header['t'] == '#commit':
                ops = body['ops']
                car_file = body['blocks']
                blocks = libipld.decode_car(car_file)
                print(ops, blocks)

    for i in range(0, 100):
        with open(os.path.join('payloads', f'{i}.txt'), 'rb') as f:
            header = libipld.decode_dag(f.read())
            print(header)


if __name__ == '__main__':
    main()
