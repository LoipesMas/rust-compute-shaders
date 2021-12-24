#version 440 
layout(local_size_x = 8 ,local_size_y = 8) in;

struct CellData{
    bool alive;
    float lifetime;
    float creation;
};

uniform vec2 u_field_size;
uniform float u_dt;
uniform float u_time;

layout(shared, binding = 0) writeonly buffer OutputData
{
    CellData next[];
};

layout(shared, binding = 1) readonly buffer InputData
{
    CellData input_data[];
};

// layout(binding = 2) uniform sampler2D u_texture; // Previous state it's texture we can sample from
int GetArrayId(ivec2 pos)
{
    return pos.x + pos.y * int(u_field_size.x);
}

void main() {
	vec4 pixel        = vec4(0.0,0.0,0.0,1.0);
	ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);

    CellData curr = input_data[GetArrayId(pixel_coord)];

    bool alive = curr.alive;

    int count = 0;
    for(int i = -1; i <= 1; ++i)
    {
        for(int j = -1; j <= 1; ++j)
        {
            if(i == 0 && j == 0)
             continue;

            if(input_data[GetArrayId(pixel_coord + ivec2(i,j))].alive)
                ++count;
        }
    }

    CellData new_cell;
    new_cell.alive = false;
    new_cell.lifetime = curr.lifetime;
    new_cell.creation = curr.creation;

    if(count < 2)                                   new_cell.alive = false;
    else if(alive && (count == 2 || count == 3))    new_cell.alive = true;
    else if(alive && count > 3)                     new_cell.alive = false;
    else if(!alive && count == 3)                   new_cell.alive = true;

    new_cell.lifetime = curr.lifetime - u_dt;
    if(new_cell.alive != curr.alive)
    {
        new_cell.lifetime = 1.0;
        new_cell.creation = u_time;
    }

    if(new_cell.lifetime < 0.0)
    {
        new_cell.alive = false;
    }


    next[GetArrayId(pixel_coord)] = new_cell;
}
