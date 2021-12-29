## Just some fun with compute shaders
Used `glow` for opengl context, `glutin` for window creation, `imgui` for ui.
### Game of Life
[Game of Life](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life) with compute shaders allows for pretty big grids (i.e. 4096x4096).  
`cargo run --bin gol --release`

### Boids
[Boids](https://en.wikipedia.org/wiki/Boids) simulation. Again, doing it on GPU allows for a larger number of agents.  
It lacks some optimizations and collision avoidance is kinda clunky, but still handles a thousand of agents.  
`cargo run --bin boid --release`

### Mold
Mold simulation, inspired by [this video](https://www.youtube.com/watch?v=X-iSQQgOd1A), which was based on [this paper](https://uwe-repository.worktribe.com/output/980579).  
Can handle a million of agents (depending on texture size and sensor size).  
`cargo run --bin mold --release`

#### Hopefully more to come??
(I want to try fluid simulation)
