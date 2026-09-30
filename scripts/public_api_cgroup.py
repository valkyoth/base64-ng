"""Verify kernel-enforced aggregate limits before executing candidate code."""

import os
from pathlib import Path
import sys


def verify(memory, root=Path('/sys/fs/cgroup'), membership=Path('/proc/self/cgroup')):
    entries = membership.read_text().splitlines()
    paths = [line[3:] for line in entries if line.startswith('0::/')]
    if len(paths) != 1 or '..' in Path(paths[0]).parts:
        raise RuntimeError('cgroup v2 membership unavailable')
    group = root / paths[0].lstrip('/')
    expected = {'memory.max': str(memory), 'memory.swap.max': '0', 'pids.max': '128'}
    for name, value in expected.items():
        if (group / name).read_text().strip() != value:
            raise RuntimeError(f'aggregate limit not enforced: {name}')
    quota, period = (group / 'cpu.max').read_text().split()
    if quota == 'max' or int(period) <= 0 or not 0 < int(quota) <= 2 * int(period):
        raise RuntimeError('aggregate CPU limit not enforced')


if __name__ == '__main__':
    verify(int(sys.argv[1]))
    os.execv(sys.argv[2], sys.argv[2:])
