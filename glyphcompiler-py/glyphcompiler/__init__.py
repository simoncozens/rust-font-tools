from .glyphcompiler import _compile as rust_compile
from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent, GlyphCoordinates
from fontTools.ttLib.tables import ttProgram


def to_component_object(c):
    r = GlyphComponent()
    r.transform = c["transform"]
    r.x = c["x"]
    r.y = c["y"]
    r.glyphName = c["glyphName"]
    return r


def to_glyph_object(d):
    if d is None:
        return None
    g = Glyph()
    if d["components"]:
        g.numberOfContours = -1
        g.components = [to_component_object(c) for c in d["components"]]
        return g
    g.numberOfContours = len(d["endPtsOfContours"])
    g.endPtsOfContours = d["endPtsOfContours"]
    g.coordinates = GlyphCoordinates(d["coordinates"])
    g.flags = bytearray(d["flags"])
    g.program = ttProgram.Program()
    g.program.fromBytecode([])
    return g


def compile(font, glyphs):
    res = rust_compile(font, glyphs)
    for glyph in res.keys():
        res[glyph] = [to_glyph_object(v) for v in res[glyph]]
    return res
