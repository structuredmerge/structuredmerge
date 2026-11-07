from pathlib import Path
import unittest


class ParserPromotionEvidenceTest(unittest.TestCase):
    path = Path(__file__).resolve().parents[2] / "contracts/PARSER_PROMOTION_EVIDENCE.md"

    def test_matrix_covers_all_candidates_and_keeps_defaults_closed(self):
        text = self.path.read_text()
        for candidate in ("ruby-prism", "oxc_parser", "swc_ecma_parser", "saphyr", "taplo", "comrak"):
            assert f"`{candidate}`" in text
        assert text.count("Deferred; no promotion") == 6
        assert "must remain out of the kernel" in text


if __name__ == "__main__":
    unittest.main()
