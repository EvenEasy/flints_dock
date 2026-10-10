"""Resize the single Flint's Dock window on the current X11 display.

Usage: python3 tests/native/resize-x11-window.py PHYSICAL_WIDTH PHYSICAL_HEIGHT
The helper does not open pages, invoke IPC, or perform wallet operations.
"""

import ctypes
import ctypes.util
import json
import sys

xlib = ctypes.CDLL(ctypes.util.find_library('X11'))
xlib.XOpenDisplay.argtypes = [ctypes.c_char_p]
xlib.XOpenDisplay.restype = ctypes.c_void_p
xlib.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
xlib.XDefaultRootWindow.restype = ctypes.c_ulong
xlib.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
xlib.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
xlib.XFree.argtypes = [ctypes.c_void_p]
xlib.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
xlib.XInternAtom.restype = ctypes.c_ulong
xlib.XGetWindowProperty.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_long, ctypes.c_long, ctypes.c_int, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_void_p)]
xlib.XResizeWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_uint, ctypes.c_uint]
xlib.XFlush.argtypes = [ctypes.c_void_p]
xlib.XCloseDisplay.argtypes = [ctypes.c_void_p]
display = xlib.XOpenDisplay(None)
if not display:
    raise RuntimeError('Could not open current X11 display')
def windows(parent, depth=0):
    name = ctypes.c_char_p()
    xlib.XFetchName(display, parent, ctypes.byref(name))
    title = name.value.decode('utf-8', 'replace') if name.value else ''
    if name:
        xlib.XFree(ctypes.cast(name, ctypes.c_void_p))
    # GTK may expose the application title only in the UTF-8 EWMH property.
    atom = xlib.XInternAtom(display, b'_NET_WM_NAME', 0)
    actual_type = ctypes.c_ulong()
    actual_format = ctypes.c_int()
    count_bytes = ctypes.c_ulong()
    remaining = ctypes.c_ulong()
    value = ctypes.c_void_p()
    if xlib.XGetWindowProperty(display, parent, atom, 0, 1024, 0, 0, ctypes.byref(actual_type), ctypes.byref(actual_format), ctypes.byref(count_bytes), ctypes.byref(remaining), ctypes.byref(value)) == 0 and value:
        if actual_format.value == 8:
            title = ctypes.string_at(value, count_bytes.value).decode('utf-8', 'replace')
        xlib.XFree(value)
    if title != 'dock-flints-app' and 'dock' in title.lower() and 'flint' in title.lower():
        yield parent, title
    if depth >= 5:
        return
    root = ctypes.c_ulong()
    owner = ctypes.c_ulong()
    children = ctypes.POINTER(ctypes.c_ulong)()
    count = ctypes.c_uint()
    if xlib.XQueryTree(display, parent, ctypes.byref(root), ctypes.byref(owner), ctypes.byref(children), ctypes.byref(count)):
        child_ids = [children[i] for i in range(count.value)]
        if children:
            xlib.XFree(children)
        for child in child_ids:
            yield from windows(child, depth + 1)
selected = list(windows(xlib.XDefaultRootWindow(display)))
if len(selected) != 1:
    raise RuntimeError(f'Expected one Flint Dock window, found {selected}')
window, title = selected[0]
width, height = map(int, sys.argv[1:3])
xlib.XResizeWindow(display, window, width, height)
xlib.XFlush(display)
xlib.XCloseDisplay(display)
print(json.dumps({'window': hex(window), 'title': title, 'physicalSize': [width, height]}))
