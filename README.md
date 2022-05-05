# Asteroids

This is my reimplementation of a classic arcade game. I built it on top of [WGPU tutorial project](https://sotrh.github.io/learn-wgpu/) to deepen my knowledge of Rust and making games.

# Why

Following an [advice from Casey Muratori](https://youtu.be/NXsWViTB238?t=4610) to start small and learn all the steps of making games, I tried to avoid googling what the preferred way of doing things is, and instead get a first-hand experience.

## What

**3D** rendering, **2D** game world.

**Wraparound** world topology: objects leaving the screen to the left appear on the right instantly. This also affects collisions and light. This was the trickiest part.

Naive **collision detection**.

[rust-gpu](https://github.com/EmbarkStudios/rust-gpu) shaders (initially WGSL).

Ad-hoc **ECS**.

Simple **font rendering**.

## Status

The goal was to learn the implications of low-level game development, and it is fulfilled. I don't have plans on developing this version further.

Initially, I planned to add explosion effects for when asteroids are destroyed, but this involved changing the rendering pipeline so I could easily add custom shaders per model. I got stuck in constant refactoring and decided to move on.