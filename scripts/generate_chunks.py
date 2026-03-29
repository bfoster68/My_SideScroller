#!/usr/bin/env python3
"""Generate updated chunks.ldtk with new IntGrid types, section field, and 14 new levels."""

import json
import uuid
import copy

INPUT_PATH = "/Users/williamfoster/Documents/GitHub/My_SideScroller/.claude/worktrees/nervous-leakey/assets/levels/chunks.ldtk"
OUTPUT_PATH = INPUT_PATH

# Entity definition UIDs
ENT = {
    "EntryPoint": 10, "ExitPoint": 11, "WalkingEnemy": 12, "Coin": 13,
    "Spike": 14, "FlyingEnemy": 15, "MovingPlatform": 16, "ShooterEnemy": 17,
    "ChargingEnemy": 18, "FlyingRangedEnemy": 19, "Saw": 20, "Powerup": 21,
    "BreakableRow": 22, "TimedTrap": 23, "BoulderSpawner": 24, "Lava": 25,
    "CoinOnPlatform": 26,
}

# Entity metadata: (width, height, pivotX, pivotY, tags, smartColor)
ENT_META = {
    "EntryPoint":       (16, 16, 0.5, 1, ["meta"], "#00FF00"),
    "ExitPoint":        (16, 16, 0.5, 1, ["meta"], "#FF0000"),
    "WalkingEnemy":     (40, 48, 0.5, 1, ["enemy"], "#FF4444"),
    "Coin":             (20, 20, 0.5, 1, ["collectible"], "#FFD700"),
    "Spike":            (24, 20, 0.5, 1, ["hazard"], "#AAAAAA"),
    "FlyingEnemy":      (36, 36, 0.5, 0.5, ["enemy"], "#FF8800"),
    "MovingPlatform":   (120, 20, 0.5, 0.5, ["platform"], "#44AAFF"),
    "ShooterEnemy":     (40, 48, 0.5, 1, ["enemy"], "#CC44CC"),
    "ChargingEnemy":    (44, 48, 0.5, 1, ["enemy"], "#DD2222"),
    "FlyingRangedEnemy":(36, 36, 0.5, 0.5, ["enemy"], "#FF44FF"),
    "Saw":              (32, 32, 0.5, 1, ["hazard"], "#888888"),
    "Powerup":          (24, 24, 0.5, 1, ["collectible"], "#44FF44"),
    "BreakableRow":     (128, 20, 0.5, 0.5, ["platform"], "#CC8844"),
    "TimedTrap":        (24, 20, 0.5, 1, ["hazard"], "#FFAA00"),
    "BoulderSpawner":   (16, 16, 0.5, 0.5, ["hazard"], "#886644"),
    "Lava":             (600, 30, 0.5, 0.5, ["hazard"], "#FF4400"),
    "CoinOnPlatform":   (20, 20, 0.5, 1, ["collectible"], "#FFDD00"),
}

# Counter for unique IDs
_iid_counter = 0
def new_iid():
    global _iid_counter
    _iid_counter += 1
    return f"e1{_iid_counter:06x}"

_level_iid_counter = 0
def new_level_iid():
    global _level_iid_counter
    _level_iid_counter += 1
    return f"b1000000-0000-0000-0000-{_level_iid_counter:012x}"

_layer_iid_counter = 0
def new_layer_iid():
    global _layer_iid_counter
    _layer_iid_counter += 1
    return f"c1000000-0000-0000-0000-{_layer_iid_counter:012x}"


def make_entity(identifier, px_x, px_y, field_instances=None, width_override=None, height_override=None):
    w, h, px, py, tags, sc = ENT_META[identifier]
    if width_override: w = width_override
    if height_override: h = height_override
    gx = px_x // 16
    gy = px_y // 16
    return {
        "__identifier": identifier,
        "__grid": [gx, gy],
        "__pivot": [px, py],
        "__tags": tags,
        "__tile": None,
        "__smartColor": sc,
        "__worldX": None,
        "__worldY": None,
        "iid": new_iid(),
        "width": w,
        "height": h,
        "defUid": ENT[identifier],
        "px": [px_x, px_y],
        "fieldInstances": field_instances or []
    }


def make_field(identifier, ftype, value, def_uid):
    return {
        "__identifier": identifier,
        "__type": ftype,
        "__value": value,
        "__tile": None,
        "defUid": def_uid,
        "realEditorValues": []
    }


def make_level(identifier, uid, px_wid, px_hei, diff_min, diff_max, has_ground, section, entities, int_grid_csv, world_x=0):
    cw = px_wid // 16
    ch = px_hei // 16

    field_instances = [
        make_field("difficulty_min", "Float", diff_min, 40),
        make_field("difficulty_max", "Float", diff_max, 41),
        make_field("has_ground", "Bool", has_ground, 42),
        make_field("section", "Int", section, 43),
    ]

    entities_layer = {
        "__identifier": "Entities",
        "__type": "Entities",
        "__cWid": cw,
        "__cHei": ch,
        "__gridSize": 16,
        "__opacity": 1,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "iid": new_layer_iid(),
        "levelId": uid,
        "layerDefUid": 1,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "intGridCsv": [],
        "autoLayerTiles": [],
        "seed": 0,
        "overrideTilesetUid": None,
        "gridTiles": [],
        "entityInstances": entities
    }

    collision_layer = {
        "__identifier": "Collision",
        "__type": "IntGrid",
        "__cWid": cw,
        "__cHei": ch,
        "__gridSize": 16,
        "__opacity": 1,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "iid": new_layer_iid(),
        "levelId": uid,
        "layerDefUid": 2,
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "intGridCsv": int_grid_csv,
        "autoLayerTiles": [],
        "seed": 0,
        "overrideTilesetUid": None,
        "gridTiles": [],
        "entityInstances": []
    }

    return {
        "identifier": identifier,
        "iid": new_level_iid(),
        "uid": uid,
        "worldX": world_x,
        "worldY": 0,
        "worldDepth": 0,
        "pxWid": px_wid,
        "pxHei": px_hei,
        "__bgColor": "#1A1A2E",
        "bgColor": None,
        "useAutoIdentifier": False,
        "bgRelPath": None,
        "bgPos": None,
        "bgPivotX": 0.5,
        "bgPivotY": 0.5,
        "__smartColor": "#ADADB5",
        "__bgPos": None,
        "__neighbours": [],
        "externalRelPath": None,
        "fieldInstances": field_instances,
        "layerInstances": [entities_layer, collision_layer]
    }


def make_grid(width_cells, height_cells):
    """Create empty grid."""
    return [[0]*width_cells for _ in range(height_cells)]


def grid_to_csv(grid):
    """Flatten 2D grid to 1D row-major array."""
    result = []
    for row in grid:
        result.extend(row)
    return result


def set_platform(grid, row, col_start, col_end, tile_type=1):
    """Set a horizontal platform in the grid."""
    for c in range(col_start, col_end):
        if 0 <= row < len(grid) and 0 <= c < len(grid[0]):
            grid[row][c] = tile_type


def set_ground(grid, col_start, col_end, tile_type=3):
    """Fill ground (bottom 2 rows)."""
    h = len(grid)
    for row in [h-2, h-1]:
        set_platform(grid, row, col_start, col_end, tile_type)


def set_wall(grid, col, row_start, row_end, tile_type=1):
    """Vertical wall."""
    for r in range(row_start, row_end):
        if 0 <= r < len(grid) and 0 <= col < len(grid[0]):
            grid[r][col] = tile_type


# ============================================================
# Level design functions
# ============================================================

def design_easy_stepping_stones():
    """800x400 - simple ascending platforms with coins, forgiving spacing."""
    w, h = 50, 25  # 800/16, 400/16
    grid = make_grid(w, h)

    # Platforms ascending from left to right
    set_platform(grid, 20, 2, 8)     # low left
    set_platform(grid, 18, 10, 16)   # mid-low
    set_platform(grid, 16, 18, 24)   # mid
    set_platform(grid, 14, 26, 32)   # mid-high
    set_platform(grid, 12, 34, 40)   # high
    set_platform(grid, 10, 42, 48)   # highest

    entities = [
        make_entity("EntryPoint", 8, 320),
        make_entity("ExitPoint", 760, 176),
        # Coins above each platform
        make_entity("Coin", 80, 288),
        make_entity("Coin", 200, 256),
        make_entity("Coin", 328, 224),
        make_entity("Coin", 464, 192),
        make_entity("Coin", 592, 160),
        make_entity("Coin", 720, 128),
        make_entity("CoinOnPlatform", 136, 288),
    ]

    return make_level("easy_stepping_stones", 100, 800, 400, 0.0, 0.4, False, None, entities, grid_to_csv(grid))


def design_easy_ground_coins():
    """600x400 - flat ground run with coin arcs, has_ground=true."""
    w, h = 37, 25  # 600/16 = 37.5, round to 37
    # Actually 600/16 = 37.5, but LDtk uses exact pixel dims. Grid is floor(600/16)=37
    # Let's use 608 to be clean: 38 cells. Actually let's keep 600 and 37 cells (some partial)
    # Better: use pxWid=608 so it divides evenly
    w, h = 38, 25  # 608/16
    px_wid = 608
    grid = make_grid(w, h)

    set_ground(grid, 0, w)

    entities = [
        make_entity("EntryPoint", 8, 368),
        make_entity("ExitPoint", px_wid - 16, 368),
        # Coin arc 1
        make_entity("Coin", 96, 320),
        make_entity("Coin", 128, 296),
        make_entity("Coin", 160, 280),
        make_entity("Coin", 192, 296),
        make_entity("Coin", 224, 320),
        # Coin arc 2
        make_entity("Coin", 320, 320),
        make_entity("Coin", 352, 296),
        make_entity("Coin", 384, 280),
        make_entity("Coin", 416, 296),
        make_entity("Coin", 448, 320),
        # Powerup in the middle
        make_entity("Powerup", 272, 360),
    ]

    return make_level("easy_ground_coins", 101, px_wid, 400, 0.0, 0.3, True, None, entities, grid_to_csv(grid))


def design_easy_bounce_intro():
    """800x400 - spring platforms teaching the mechanic."""
    w, h = 50, 25
    grid = make_grid(w, h)

    # Entry platform
    set_platform(grid, 20, 1, 6)
    # Spring platforms (type 8)
    set_platform(grid, 21, 10, 13, 8)  # spring
    set_platform(grid, 21, 22, 25, 8)  # spring
    set_platform(grid, 21, 34, 37, 8)  # spring
    # Landing platforms
    set_platform(grid, 16, 14, 20)
    set_platform(grid, 12, 26, 32)
    set_platform(grid, 8, 38, 44)
    # Exit platform
    set_platform(grid, 8, 45, 49)

    entities = [
        make_entity("EntryPoint", 24, 320),
        make_entity("ExitPoint", 776, 128),
        # Coins above spring arcs
        make_entity("Coin", 176, 192),
        make_entity("Coin", 240, 128),
        make_entity("Coin", 368, 160),
        make_entity("Coin", 464, 96),
        make_entity("Coin", 576, 64),
        make_entity("CoinOnPlatform", 272, 192),
        make_entity("CoinOnPlatform", 464, 128),
    ]

    return make_level("easy_bounce_intro", 102, 800, 400, 0.0, 0.4, False, None, entities, grid_to_csv(grid))


def design_medium_ice_slide():
    """1000x400 - ice platforms descending, coins at bottom, section=1."""
    w, h = 62, 25  # 1000/16=62.5 -> use 992 px = 62 cells
    px_wid = 992
    grid = make_grid(w, h)

    # Descending ice platforms
    set_platform(grid, 8, 2, 10, 6)    # ice high left
    set_platform(grid, 11, 12, 20, 6)  # ice
    set_platform(grid, 14, 22, 30, 6)  # ice
    set_platform(grid, 17, 32, 40, 6)  # ice
    set_platform(grid, 20, 42, 50, 6)  # ice low
    # Exit platform (solid)
    set_platform(grid, 20, 54, 61)

    entities = [
        make_entity("EntryPoint", 24, 128),
        make_entity("ExitPoint", px_wid - 24, 320),
        # Coins along the slide
        make_entity("Coin", 248, 144),
        make_entity("Coin", 408, 192),
        make_entity("Coin", 568, 256),
        make_entity("Coin", 728, 288),
        # Coins at bottom
        make_entity("Coin", 808, 304),
        make_entity("Coin", 840, 304),
        make_entity("Coin", 872, 304),
    ]

    return make_level("medium_ice_slide", 103, px_wid, 400, 0.3, 0.7, False, 1, entities, grid_to_csv(grid))


def design_medium_conveyor_maze():
    """1000x400 - conveyor platforms pushing player."""
    w, h = 62, 25
    px_wid = 992
    grid = make_grid(w, h)

    # Entry platform
    set_platform(grid, 18, 1, 6)
    # Conveyor sections pushing right (player must go right, but some push wrong way)
    set_platform(grid, 18, 8, 18, 5)    # conveyor
    set_platform(grid, 14, 20, 30, 5)   # conveyor higher
    set_platform(grid, 18, 32, 42, 5)   # conveyor
    set_platform(grid, 14, 44, 54, 5)   # conveyor higher
    # Exit platform
    set_platform(grid, 14, 56, 62)
    # Some solid stepping stones between conveyors
    set_platform(grid, 16, 18, 21)
    set_platform(grid, 16, 42, 45)

    entities = [
        make_entity("EntryPoint", 16, 288),
        make_entity("ExitPoint", px_wid - 16, 224),
        make_entity("Coin", 200, 256),
        make_entity("Coin", 400, 192),
        make_entity("Coin", 600, 256),
        make_entity("Coin", 784, 192),
        make_entity("WalkingEnemy", 240, 288, [
            make_field("patrol_width", "Float", 80, 30)
        ]),
    ]

    return make_level("medium_conveyor_maze", 104, px_wid, 400, 0.3, 0.7, False, None, entities, grid_to_csv(grid))


def design_medium_crumble_run():
    """800x400 - crumbling platforms in a row, must keep moving."""
    w, h = 50, 25
    grid = make_grid(w, h)

    # Entry solid platform
    set_platform(grid, 18, 1, 5)
    # Crumbling platform chain (type 7)
    set_platform(grid, 18, 7, 11, 7)
    set_platform(grid, 18, 13, 17, 7)
    set_platform(grid, 18, 19, 23, 7)
    set_platform(grid, 18, 25, 29, 7)
    set_platform(grid, 18, 31, 35, 7)
    set_platform(grid, 18, 37, 41, 7)
    # Exit solid platform
    set_platform(grid, 18, 43, 49)

    entities = [
        make_entity("EntryPoint", 16, 288),
        make_entity("ExitPoint", 776, 288),
        # Coins above crumbling platforms
        make_entity("Coin", 144, 256),
        make_entity("Coin", 240, 256),
        make_entity("Coin", 336, 256),
        make_entity("Coin", 432, 256),
        make_entity("Coin", 528, 256),
        make_entity("Coin", 624, 256),
        make_entity("Powerup", 384, 240),
    ]

    return make_level("medium_crumble_run", 105, 800, 400, 0.3, 0.7, False, None, entities, grid_to_csv(grid))


def design_medium_mixed_challenge():
    """1200x400 - mix of one-way, regular, and breakable with enemies."""
    w, h = 75, 25
    grid = make_grid(w, h)

    # Ground start
    set_platform(grid, 20, 1, 8)
    # One-way platforms (type 4)
    set_platform(grid, 16, 10, 16, 4)
    set_platform(grid, 12, 18, 24, 4)
    # Solid platform
    set_platform(grid, 14, 26, 32)
    # Breakable section (type 2)
    set_platform(grid, 14, 34, 42, 2)
    # One-way platforms
    set_platform(grid, 16, 44, 50, 4)
    set_platform(grid, 18, 52, 58)
    # Exit platform
    set_platform(grid, 18, 62, 74)
    # Some walls for variety
    set_wall(grid, 60, 14, 20)

    entities = [
        make_entity("EntryPoint", 16, 320),
        make_entity("ExitPoint", 1184, 288),
        make_entity("WalkingEnemy", 456, 224, [
            make_field("patrol_width", "Float", 120, 30)
        ]),
        make_entity("FlyingEnemy", 600, 160),
        make_entity("Coin", 200, 224),
        make_entity("Coin", 336, 160),
        make_entity("Coin", 504, 192),
        make_entity("Coin", 744, 224),
        make_entity("Coin", 920, 256),
        make_entity("Spike", 552, 224),
        make_entity("BreakableRow", 608, 224, [
            make_field("block_count", "Int", 5, 35)
        ], width_override=128),
    ]

    return make_level("medium_mixed_challenge", 106, 1200, 400, 0.3, 0.7, False, None, entities, grid_to_csv(grid))


def design_hard_ice_enemy_gauntlet():
    """1200x400 - ice platforms with shooter enemies, section=1."""
    w, h = 75, 25
    grid = make_grid(w, h)

    # Ice platform chain with shooters
    set_platform(grid, 18, 1, 8, 6)     # ice entry
    set_platform(grid, 14, 12, 22, 6)   # ice mid-high
    set_platform(grid, 18, 24, 34, 6)   # ice mid-low
    set_platform(grid, 10, 36, 44, 6)   # ice high
    set_platform(grid, 18, 46, 56, 6)   # ice low
    set_platform(grid, 14, 58, 68, 6)   # ice mid
    # Exit solid
    set_platform(grid, 14, 70, 75)

    entities = [
        make_entity("EntryPoint", 16, 288),
        make_entity("ExitPoint", 1184, 224),
        make_entity("ShooterEnemy", 280, 224),
        make_entity("ShooterEnemy", 640, 160),
        make_entity("ShooterEnemy", 1008, 224),
        make_entity("FlyingEnemy", 440, 120),
        make_entity("Coin", 200, 192),
        make_entity("Coin", 464, 256),
        make_entity("Coin", 800, 256),
        make_entity("Coin", 1040, 192),
        make_entity("Powerup", 624, 128),
    ]

    return make_level("hard_ice_enemy_gauntlet", 107, 1200, 400, 0.5, 1.0, False, 1, entities, grid_to_csv(grid))


def design_hard_conveyor_saw():
    """1000x400 - conveyors pushing toward saws."""
    w, h = 62, 25
    px_wid = 992
    grid = make_grid(w, h)

    # Entry
    set_platform(grid, 18, 1, 6)
    # Conveyor pushing right toward saw
    set_platform(grid, 18, 8, 20, 5)
    # Gap, then conveyor pushing toward another saw
    set_platform(grid, 14, 22, 34, 5)
    # More conveyors
    set_platform(grid, 18, 36, 48, 5)
    # Exit
    set_platform(grid, 14, 52, 62)
    # Solid stepping stones to escape
    set_platform(grid, 14, 18, 21)
    set_platform(grid, 10, 32, 35)
    set_platform(grid, 14, 46, 49)

    entities = [
        make_entity("EntryPoint", 16, 288),
        make_entity("ExitPoint", px_wid - 16, 224),
        # Saws at end of conveyors
        make_entity("Saw", 312, 288, [
            make_field("patrol_width", "Float", 0, 34)
        ]),
        make_entity("Saw", 536, 224, [
            make_field("patrol_width", "Float", 0, 34)
        ]),
        make_entity("Saw", 760, 288, [
            make_field("patrol_width", "Float", 0, 34)
        ]),
        make_entity("Coin", 160, 256),
        make_entity("Coin", 416, 192),
        make_entity("Coin", 656, 256),
        make_entity("FlyingEnemy", 280, 120),
        make_entity("FlyingEnemy", 520, 80),
    ]

    return make_level("hard_conveyor_saw", 108, px_wid, 400, 0.5, 1.0, False, None, entities, grid_to_csv(grid))


def design_hard_crumble_ascent():
    """1000x400 - ascending crumbling platforms with flying enemies."""
    w, h = 62, 25
    px_wid = 992
    grid = make_grid(w, h)

    # Entry
    set_platform(grid, 22, 1, 6)
    # Ascending crumbling platforms (type 7)
    set_platform(grid, 20, 8, 13, 7)
    set_platform(grid, 17, 15, 20, 7)
    set_platform(grid, 14, 22, 27, 7)
    set_platform(grid, 11, 29, 34, 7)
    set_platform(grid, 8, 36, 41, 7)
    set_platform(grid, 5, 43, 48, 7)
    # Descending to exit
    set_platform(grid, 8, 50, 55, 7)
    set_platform(grid, 12, 55, 60)  # solid exit

    entities = [
        make_entity("EntryPoint", 16, 352),
        make_entity("ExitPoint", px_wid - 48, 192),
        make_entity("FlyingEnemy", 200, 248),
        make_entity("FlyingEnemy", 360, 152),
        make_entity("FlyingRangedEnemy", 520, 80),
        make_entity("FlyingEnemy", 680, 56),
        make_entity("Coin", 168, 288),
        make_entity("Coin", 280, 224),
        make_entity("Coin", 392, 160),
        make_entity("Coin", 504, 96),
        make_entity("Coin", 640, 48),
        make_entity("Powerup", 392, 128),
    ]

    return make_level("hard_crumble_ascent", 109, px_wid, 400, 0.5, 1.0, False, None, entities, grid_to_csv(grid))


def design_hard_spring_gauntlet():
    """1200x400 - spring chain with flying ranged enemies."""
    w, h = 75, 25
    grid = make_grid(w, h)

    # Entry
    set_platform(grid, 20, 1, 5)
    # Spring chain (type 8)
    set_platform(grid, 22, 8, 11, 8)
    set_platform(grid, 22, 18, 21, 8)
    set_platform(grid, 22, 28, 31, 8)
    set_platform(grid, 22, 38, 41, 8)
    set_platform(grid, 22, 48, 51, 8)
    set_platform(grid, 22, 58, 61, 8)
    # Landing platforms between springs
    set_platform(grid, 14, 12, 17)
    set_platform(grid, 14, 22, 27)
    set_platform(grid, 14, 32, 37)
    set_platform(grid, 14, 42, 47)
    set_platform(grid, 14, 52, 57)
    # Exit
    set_platform(grid, 14, 64, 74)

    entities = [
        make_entity("EntryPoint", 16, 320),
        make_entity("ExitPoint", 1184, 224),
        # Flying ranged enemies between bounces
        make_entity("FlyingRangedEnemy", 232, 120),
        make_entity("FlyingRangedEnemy", 440, 100),
        make_entity("FlyingRangedEnemy", 640, 120),
        make_entity("FlyingRangedEnemy", 840, 100),
        make_entity("FlyingEnemy", 336, 160),
        make_entity("FlyingEnemy", 736, 160),
        make_entity("Coin", 152, 256),
        make_entity("Coin", 312, 256),
        make_entity("Coin", 472, 256),
        make_entity("Coin", 632, 256),
        make_entity("Coin", 792, 256),
        make_entity("Coin", 952, 256),
        make_entity("Powerup", 552, 128),
    ]

    return make_level("hard_spring_gauntlet", 110, 1200, 400, 0.5, 1.0, False, None, entities, grid_to_csv(grid))


def design_hard_everything():
    """1600x400 - all platform types mixed, difficulty 0.7-1.0."""
    w, h = 100, 25
    grid = make_grid(w, h)

    # Entry solid
    set_platform(grid, 20, 1, 6)
    # Ice section
    set_platform(grid, 18, 8, 16, 6)
    # One-way platforms
    set_platform(grid, 14, 18, 24, 4)
    # Conveyor section
    set_platform(grid, 18, 26, 36, 5)
    # Crumbling bridge
    set_platform(grid, 14, 38, 48, 7)
    # Spring section
    set_platform(grid, 22, 50, 53, 8)
    set_platform(grid, 16, 54, 58)
    set_platform(grid, 22, 60, 63, 8)
    set_platform(grid, 12, 64, 68)
    # Breakable
    set_platform(grid, 16, 70, 78, 2)
    # More ice
    set_platform(grid, 18, 80, 88, 6)
    # Exit solid
    set_platform(grid, 18, 92, 100)

    entities = [
        make_entity("EntryPoint", 16, 320),
        make_entity("ExitPoint", 1584, 288),
        # Enemies spread throughout
        make_entity("WalkingEnemy", 192, 288, [
            make_field("patrol_width", "Float", 100, 30)
        ]),
        make_entity("ShooterEnemy", 488, 288),
        make_entity("FlyingEnemy", 360, 120),
        make_entity("FlyingRangedEnemy", 680, 80),
        make_entity("FlyingRangedEnemy", 1000, 100),
        make_entity("ChargingEnemy", 1360, 288, [
            make_field("patrol_width", "Float", 120, 33)
        ]),
        # Hazards
        make_entity("Saw", 560, 224, [
            make_field("patrol_width", "Float", 60, 34)
        ]),
        make_entity("Spike", 712, 224),
        make_entity("TimedTrap", 1136, 288),
        # Collectibles
        make_entity("Coin", 160, 256),
        make_entity("Coin", 328, 192),
        make_entity("Coin", 496, 256),
        make_entity("Coin", 688, 192),
        make_entity("Coin", 880, 160),
        make_entity("Coin", 1056, 96),
        make_entity("Coin", 1200, 224),
        make_entity("Coin", 1424, 256),
        make_entity("Powerup", 896, 128),
    ]

    return make_level("hard_everything", 111, 1600, 400, 0.7, 1.0, False, None, entities, grid_to_csv(grid))


def design_volcanic_platforms():
    """1000x400 - crumbling + lava below, section=2."""
    w, h = 62, 25
    px_wid = 992
    grid = make_grid(w, h)

    # Entry solid
    set_platform(grid, 16, 1, 6)
    # Crumbling platforms over lava
    set_platform(grid, 16, 10, 16, 7)
    set_platform(grid, 14, 18, 24, 7)
    set_platform(grid, 16, 26, 32, 7)
    set_platform(grid, 12, 34, 40)  # solid safe zone
    set_platform(grid, 16, 42, 48, 7)
    set_platform(grid, 14, 50, 56, 7)
    # Exit
    set_platform(grid, 14, 58, 62)

    entities = [
        make_entity("EntryPoint", 16, 256),
        make_entity("ExitPoint", px_wid - 16, 224),
        # Lava at bottom
        make_entity("Lava", px_wid // 2, 388, width_override=px_wid - 32),
        # Enemies
        make_entity("FlyingEnemy", 280, 160),
        make_entity("FlyingEnemy", 520, 120),
        make_entity("BoulderSpawner", 560, 48),
        # Coins
        make_entity("Coin", 200, 224),
        make_entity("Coin", 336, 192),
        make_entity("Coin", 456, 224),
        make_entity("Coin", 696, 224),
        make_entity("Coin", 840, 192),
        make_entity("Powerup", 584, 176),
    ]

    return make_level("volcanic_platforms", 112, px_wid, 400, 0.5, 1.0, False, 2, entities, grid_to_csv(grid))


def design_sky_bouncing():
    """1000x400 - spring platforms ascending high, section=3."""
    w, h = 62, 25
    px_wid = 992
    grid = make_grid(w, h)

    # Entry low
    set_platform(grid, 22, 1, 6)
    # Springs ascending
    set_platform(grid, 23, 8, 11, 8)
    set_platform(grid, 18, 13, 17)   # landing
    set_platform(grid, 19, 19, 22, 8)
    set_platform(grid, 13, 24, 28)   # landing
    set_platform(grid, 14, 30, 33, 8)
    set_platform(grid, 8, 35, 39)    # landing high
    set_platform(grid, 9, 41, 44, 8)
    set_platform(grid, 4, 46, 50)    # very high landing
    # Descend to exit
    set_platform(grid, 8, 52, 56)
    set_platform(grid, 13, 56, 60)
    set_platform(grid, 18, 58, 62)

    entities = [
        make_entity("EntryPoint", 16, 352),
        make_entity("ExitPoint", px_wid - 24, 288),
        make_entity("FlyingEnemy", 264, 200),
        make_entity("FlyingRangedEnemy", 456, 80),
        make_entity("FlyingEnemy", 648, 40),
        # Coins along the path
        make_entity("Coin", 152, 304),
        make_entity("Coin", 240, 240),
        make_entity("Coin", 336, 160),
        make_entity("Coin", 480, 96),
        make_entity("Coin", 592, 32),
        make_entity("Coin", 760, 48),
        make_entity("Powerup", 392, 64),
    ]

    return make_level("sky_bouncing", 113, px_wid, 400, 0.4, 0.9, False, 3, entities, grid_to_csv(grid))


def main():
    with open(INPUT_PATH, "r") as f:
        data = json.load(f)

    # 1. Add new IntGrid values (4-8)
    collision_layer_def = None
    for layer_def in data["defs"]["layers"]:
        if layer_def["identifier"] == "Collision":
            collision_layer_def = layer_def
            break

    existing_values = {v["value"] for v in collision_layer_def["intGridValues"]}
    new_int_grid_values = [
        {"value": 4, "identifier": "OneWay", "color": "#6688CC", "tile": None, "groupUid": 0},
        {"value": 5, "identifier": "Conveyor", "color": "#55AA55", "tile": None, "groupUid": 0},
        {"value": 6, "identifier": "Ice", "color": "#AADDFF", "tile": None, "groupUid": 0},
        {"value": 7, "identifier": "Crumbling", "color": "#AA8866", "tile": None, "groupUid": 0},
        {"value": 8, "identifier": "Spring", "color": "#44CC44", "tile": None, "groupUid": 0},
    ]
    for nv in new_int_grid_values:
        if nv["value"] not in existing_values:
            collision_layer_def["intGridValues"].append(nv)

    # 2. Add "section" field to levelFields if not present
    section_exists = any(f["identifier"] == "section" for f in data["defs"]["levelFields"])
    if not section_exists:
        section_field = {
            "identifier": "section",
            "doc": "Thematic section ID (null=any, 1=ice, 2=volcanic, 3=sky)",
            "__type": "Int",
            "uid": 43,
            "type": "F_Int",
            "isArray": False,
            "canBeNull": True,
            "arrayMinLength": None,
            "arrayMaxLength": None,
            "editorDisplayMode": "ValueOnly",
            "editorDisplayScale": 1,
            "editorDisplayPos": "Above",
            "editorLinkStyle": "StraightArrow",
            "editorDisplayColor": None,
            "editorAlwaysShow": True,
            "editorShowInWorld": False,
            "editorCutLongValues": True,
            "editorTextSuffix": None,
            "editorTextPrefix": None,
            "useForSmartColor": False,
            "exportToToc": False,
            "searchable": False,
            "min": None,
            "max": None,
            "regex": None,
            "acceptFileTypes": None,
            "defaultOverride": None,
            "textLanguageMode": None,
            "symmetricalRef": False,
            "autoChainRef": True,
            "allowOutOfLevelRef": True,
            "allowedRefs": "Any",
            "allowedRefsEntityUid": None,
            "allowedRefTags": [],
            "tilesetUid": None
        }
        data["defs"]["levelFields"].append(section_field)

    # 3. Add section field to existing levels (null value)
    for level in data["levels"]:
        has_section = any(fi["__identifier"] == "section" for fi in level["fieldInstances"])
        if not has_section:
            level["fieldInstances"].append(
                make_field("section", "Int", None, 43)
            )

    # 4. Update nextUid to be above all our new UIDs
    data["nextUid"] = 200

    # 5. Generate new levels
    new_levels = [
        design_easy_stepping_stones(),
        design_easy_ground_coins(),
        design_easy_bounce_intro(),
        design_medium_ice_slide(),
        design_medium_conveyor_maze(),
        design_medium_crumble_run(),
        design_medium_mixed_challenge(),
        design_hard_ice_enemy_gauntlet(),
        design_hard_conveyor_saw(),
        design_hard_crumble_ascent(),
        design_hard_spring_gauntlet(),
        design_hard_everything(),
        design_volcanic_platforms(),
        design_sky_bouncing(),
    ]

    # Calculate worldX positions for new levels
    # Existing levels end at worldX of last level + its width
    last_world_x = 0
    for lvl in data["levels"]:
        end = lvl["worldX"] + lvl["pxWid"]
        if end > last_world_x:
            last_world_x = end

    for lvl in new_levels:
        lvl["worldX"] = last_world_x
        last_world_x += lvl["pxWid"]

    data["levels"].extend(new_levels)

    # 6. Write output
    with open(OUTPUT_PATH, "w") as f:
        json.dump(data, f, indent=2)

    print(f"Generated {len(new_levels)} new levels.")
    print(f"Total levels: {len(data['levels'])}")
    print(f"IntGrid values: {[v['identifier'] for v in collision_layer_def['intGridValues']]}")
    print(f"Level fields: {[f['identifier'] for f in data['defs']['levelFields']]}")
    print(f"Output written to: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
