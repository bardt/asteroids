# Assets

## Organization

All assets are stored in a single Blender file. Mesh name should match object name in the Blender inspector:

```
Scene Collection
    |> Collection
        |> Asteroid
            |> Asteroid
        |> Spaceship
            |> Spaceship


```

This way it will be possible to refer to the Spaceship object in the code by it's name.

## Exporting

Export all assets to a single .obj file. Z is up, Y is forward.