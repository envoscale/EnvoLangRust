"""A sampling profiler for envoc-built Windows programs.

    python winprof.py <program.map> <program.exe> [args...]

Runs the program, pauses its thread about once a millisecond, reads the instruction
pointer, and maps it to the function it is in using the `-map` file envoc writes
(`envoc build ... -map program.map`). Prints the functions where the samples fell.
Only the standard library is used (ctypes).
"""
import bisect
import ctypes
import ctypes.wintypes as wt
import subprocess
import sys
import time

k32 = ctypes.WinDLL('kernel32', use_last_error=True)
psapi = ctypes.WinDLL('psapi', use_last_error=True)
winmm = ctypes.WinDLL('winmm')

TH32CS_SNAPTHREAD = 0x4
THREAD_ALL = 0x1F03FF
CONTEXT_CONTROL = 0x100001


class THREADENTRY32(ctypes.Structure):
    _fields_ = [('dwSize', wt.DWORD), ('cntUsage', wt.DWORD), ('th32ThreadID', wt.DWORD),
                ('th32OwnerProcessID', wt.DWORD), ('tpBasePri', wt.LONG), ('tpDeltaPri', wt.LONG),
                ('dwFlags', wt.DWORD)]


def main_thread(pid):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
    te = THREADENTRY32()
    te.dwSize = ctypes.sizeof(te)
    ok = k32.Thread32First(snap, ctypes.byref(te))
    while ok:
        if te.th32OwnerProcessID == pid:
            k32.CloseHandle(snap)
            return te.th32ThreadID
        ok = k32.Thread32Next(snap, ctypes.byref(te))
    k32.CloseHandle(snap)
    return None


def module_base(hproc):
    mods = (ctypes.c_void_p * 64)()
    need = wt.DWORD()
    for _ in range(200):
        if psapi.EnumProcessModulesEx(hproc, mods, ctypes.sizeof(mods), ctypes.byref(need), 3) and mods[0]:
            return mods[0]
        time.sleep(0.001)
    return None


def main():
    mapfile, argv = sys.argv[1], sys.argv[2:]
    funcs = []
    for line in open(mapfile):
        parts = line.split()
        if len(parts) >= 2:
            funcs.append((int(parts[0], 16) - 0x400000, parts[1]))
    funcs.sort()
    addrs = [a for a, _ in funcs]
    ctx_buf = ctypes.create_string_buffer(1232 + 16)
    ctx_addr = (ctypes.addressof(ctx_buf) + 15) & ~15
    winmm.timeBeginPeriod(1)
    p = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    hproc = int(p._handle)
    tid = None
    while tid is None and p.poll() is None:
        tid = main_thread(p.pid)
    base = module_base(hproc)
    hthread = k32.OpenThread(THREAD_ALL, False, tid)
    counts = {}
    total = 0
    while p.poll() is None:
        time.sleep(0.001)
        if k32.SuspendThread(hthread) == 0xFFFFFFFF:
            break
        ctypes.memset(ctx_addr, 0, 1232)
        ctypes.c_uint32.from_address(ctx_addr + 0x30).value = CONTEXT_CONTROL
        if k32.GetThreadContext(hthread, ctypes.c_void_p(ctx_addr)):
            rip = ctypes.c_uint64.from_address(ctx_addr + 0xF8).value
            off = rip - base
            i = bisect.bisect_right(addrs, off) - 1
            name = funcs[i][1] if i >= 0 and off < 0x10000000 else '(outside: system or runtime)'
            counts[name] = counts.get(name, 0) + 1
            total += 1
        k32.ResumeThread(hthread)
    winmm.timeEndPeriod(1)
    print(f'{total} samples')
    for name, n in sorted(counts.items(), key=lambda x: -x[1])[:int(40)]:
        print(f'{100.0 * n / total:6.2f}%  {n:6d}  {name}')


main()
