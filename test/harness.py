import argparse
import pytest
import tempfile
import pathlib
import subprocess
import sys
import io
import json
from jfont import TTJ
from fontTools.ttLib import TTFont
import os
from recursive_diff import recursive_eq


environment = os.environ.get("ENVIRONMENT", "debug")
fonticulus = pathlib.Path(__file__).parent.parent / "target" / environment / "fonticulus"
home_dir = pathlib.Path(__file__).parent.resolve()
test_files = [pytest.param(x, id=x.name) for x in (home_dir / 'sources').glob("*.*")]

def clean_font(ttjfont):
    for t in ["GSUB", "GDEF", "GPOS", "HVAR", "VVAR"]:
        if t in ttjfont:
            del ttjfont[t]
    del ttjfont["head"]["checkSumAdjustment"]
    del ttjfont["head"]["modified"]

    if "gvar" in ttjfont:
        del ttjfont["gvar"] # FOR NOW
    return ttjfont

def get_expectation(source: pathlib.Path):
    expectation_file = home_dir / "expectation" / (pathlib.Path(source).name + ".expected")
    expectation_file.parent.mkdir(exist_ok=True)
    if not expectation_file.exists():
        # Compile with fontmake
        if source.suffix == ".glyphs":
            build_arg = "-o variable -g"
        elif source.suffix == ".ufo":
            build_arg = "-o ttf -u"
        elif source.suffix == ".designspace":
            build_arg = "-o variable -m"
        else:
            raise ValueError("Unknown source file type")
        with tempfile.NamedTemporaryFile() as tf:
            cmd = f"fontmake {build_arg} {source} --keep-overlaps --output-path {tf.name}"
            subprocess.run(cmd, shell=True, check=True)
            ttjfont = TTJ(TTFont(tf))
        as_json = json.dumps(ttjfont, indent=4)
        with expectation_file.open("w") as f:
            f.write(as_json)
    with expectation_file.open() as f:
        from_json = json.load(f)
    return clean_font(from_json)

@pytest.mark.parametrize("source", test_files)
def test_build(source):
    expected = get_expectation(source)
    try:
        out = subprocess.run([fonticulus, source], check=True, capture_output=True)
    except subprocess.CalledProcessError as e:
        print(e.stderr, file=sys.stderr)
        assert False
    ttjfont = json.loads(json.dumps(clean_font(TTJ(TTFont(io.BytesIO(out.stdout))))))
    recursive_eq(ttjfont, expected)
