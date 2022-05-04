A tile map rendering plugin for bevy that supports:

1. Multiple tilesheets in a single layer (every tilesheet needs to have the same resolution per sprite). (See example `layers`)
2. Transformation on sprites on each tile. (Translate, Rotate, Scale) (See example `sprite_mod`)
3. Solid color tiles and color masks for sprites. (See example `color_tile` and `sprite_mod`)
4. A component that maps to a set of tiles. (See example `as_tiles`)
5. Inbuilt mouse tile picking. (See example `interaction`)

**IMPORTANT: Tiles in this plugin are indexed with UVec3's.**

See the `minimal` example for a starting point. In that example you can use the arrow keys to change which sprite is being rendered.