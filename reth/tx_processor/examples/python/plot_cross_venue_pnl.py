import sys
import os
import pandas as pd
import matplotlib.pyplot as plt

def main(csv_path: str):
    df = pd.read_csv(csv_path)
    # Ensure sorting by block
    df = df.sort_values(['symbol', 'block'])

    fig, ax = plt.subplots(figsize=(10, 5))
    # Prefer exec-clamped PnL if present, otherwise fallback to raw
    ycol = 'pnl_usd_exec' if 'pnl_usd_exec' in df.columns else 'pnl_usd'
    for sym, g in df.groupby('symbol'):
        ax.plot(g['block'], g[ycol], label=sym)

    ax.axhline(0, color='k', linewidth=0.8)
    suffix = ' (exec-clamped)' if ycol == 'pnl_usd_exec' else ' (raw)'
    ax.set_title('Best Cross-Venue Net PnL per Block (USD approx)' + suffix)
    ax.set_xlabel('Block')
    ax.set_ylabel('PnL (USD)')
    ax.legend()
    ax.grid(True, alpha=0.3)

    out_dir = os.path.dirname(csv_path) or '.'
    out_png = os.path.join(out_dir, os.path.splitext(os.path.basename(csv_path))[0] + '.png')
    fig.tight_layout()
    fig.savefig(out_png, dpi=150)
    print(f'Wrote {out_png}')

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print('Usage: python plot_cross_venue_pnl.py <csv_path>')
        sys.exit(1)
    main(sys.argv[1])
