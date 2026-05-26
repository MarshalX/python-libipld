"""Render bar charts from pytest-benchmark JSON output.

Usage (run from this directory):
    ./run.sh                                                         # full pipeline
    uv run --with-requirements requirements.txt python chart.py results.json
"""

import json
import sys
from pathlib import Path

import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
from matplotlib.ticker import FuncFormatter

BASELINE = 'dag_cbor'
HUE_ORDER = ['libipld', 'cbrrr', 'py-ipld-dag', 'dag_cbor']


def load(path):
    with open(path) as f:
        data = json.load(f)

    rows = []
    for b in data['benchmarks']:
        info = b['extra_info']
        lib = info['lib']
        ver = info.get('version')
        rows.append(
            {
                'op': info['op'],
                'fixture': info['fixture'],
                'lib': lib,
                'lib_label': f'{lib} {ver}' if ver else lib,
                'ops_per_sec': 1.0 / b['stats']['mean'],
            }
        )

    return pd.DataFrame(rows)


def add_relative(df, baseline):
    out = []
    for (_, _), group in df.groupby(['op', 'fixture']):
        # baseline ops/sec for this (op, fixture); fall back to slowest lib if missing
        if baseline in group['lib'].values:
            base = group.loc[group['lib'] == baseline, 'ops_per_sec'].iloc[0]
        else:
            base = group['ops_per_sec'].min()

        for _, r in group.iterrows():
            out.append({**r.to_dict(), 'rel': r['ops_per_sec'] / base})

    return pd.DataFrame(out)


def plot(df, op, baseline, title, outfile):
    sub = df[df['op'] == op].copy()
    if sub.empty:
        print(f'skipping {outfile}: no {op} results')
        return

    # preserve canonical lib ordering, but resolve to versioned labels for the legend
    label_for_lib = dict(zip(sub['lib'], sub['lib_label']))
    labels_present = [label_for_lib[lib] for lib in HUE_ORDER if lib in label_for_lib]

    sns.set_theme(style='darkgrid')
    fig, ax = plt.subplots(figsize=(10, 7))
    sns.barplot(
        data=sub,
        x='fixture',
        y='rel',
        hue='lib_label',
        hue_order=labels_present,
        ax=ax,
    )

    ax.axhline(1.0, color='gray', linestyle='--', linewidth=1)
    ax.set_title(title)
    ax.set_xlabel('Document')
    ax.set_ylabel(f'Operations/second relative to {baseline}')
    ax.yaxis.set_major_formatter(FuncFormatter(lambda y, _: f'{int(y)}x'))
    ax.legend(title='library')

    fig.tight_layout()
    fig.savefig(outfile, dpi=150)
    print(f'wrote {outfile}')


def main():
    path = Path(sys.argv[1] if len(sys.argv) > 1 else 'results.json')

    df = load(path)
    baseline = BASELINE if BASELINE in df['lib'].values else df['lib'].iloc[0]
    df = add_relative(df, baseline)

    here = Path(__file__).parent
    plot(df, 'decode', baseline, 'deserialization', here / 'deserialization.png')
    plot(df, 'encode', baseline, 'serialization', here / 'serialization.png')


if __name__ == '__main__':
    main()
