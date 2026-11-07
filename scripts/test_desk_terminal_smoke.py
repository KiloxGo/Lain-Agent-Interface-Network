"""Test-only terminal decoder compatibility; needs native acceptance dependencies."""
import unittest
try:
    import pyte
except ImportError:
    pyte=None
from desk_terminal_smoke import screen_text

@unittest.skipIf(pyte is None,"pyte is installed with native terminal acceptance tools")
class ScreenTextTests(unittest.TestCase):
    def test_orphan_wide_stub_keeps_a_blank_physical_column(self):
        screen=pyte.Screen(6,2)
        stream=pyte.Stream(screen)
        stream.feed("中")
        self.assertEqual(screen_text(screen),"中    \n      ")
        stream.feed("\x1b[1;1HA")
        self.assertEqual(screen_text(screen),"A     \n      ")
        stream.feed("\x1b[1;2HB")
        self.assertEqual(screen_text(screen),"AB    \n      ")

if __name__=="__main__":unittest.main()
