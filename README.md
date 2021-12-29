## Just some fun with compute shaders
Used `glow` for opengl context, `glutin` for window creation, `imgui` for ui.
### Game of Life
[Game of Life](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life) with compute shaders allows for pretty big grids (i.e. 4096x4096).  
`cargo run --bin gol --release`
![gol](https://user-images.githubusercontent.com/46327403/147675131-4ea304c2-e76b-436d-b558-10bdcc5f1609.png)


### Boids
[Boids](https://en.wikipedia.org/wiki/Boids) simulation. Again, doing it on GPU allows for a larger number of agents.  
It lacks some optimizations and collision avoidance is kinda clunky, but still handles a thousand of agents.  
`cargo run --bin boid --release`


https://user-images.githubusercontent.com/46327403/147675185-8cbfe4c9-485d-410a-b69d-499e95dd08c4.mp4


### Mold
Mold simulation, inspired by [this video](https://www.youtube.com/watch?v=X-iSQQgOd1A), which was based on [this paper](https://uwe-repository.worktribe.com/output/980579).  
Can handle a million of agents (depending on texture size and sensor size).  
`cargo run --bin mold --release`
![ksnip_20211228-174219](https://user-images.githubusercontent.com/46327403/147675498-75699eab-236b-42f4-b2c2-6347585650f2.png)

https://user-images.githubusercontent.com/46327403/147675343-94d6823d-75d7-47a0-8eda-513746fdf7fc.mp4



#### Hopefully more to come??
(I want to try fluid simulation)
