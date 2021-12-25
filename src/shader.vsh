#version 440 
layout(local_size_x = 8 ,local_size_y = 8) in;

struct CellData{
	float val;
};

uniform vec2 u_field_size;

layout(shared, binding = 0) writeonly buffer OutputData
{
    CellData next[];
};

layout(shared, binding = 1) readonly buffer InputData
{
    CellData input_data[];
};

layout(binding = 2, rgba32f) uniform image2D u_texture; 
int GetArrayId(ivec2 pos)
{
    return pos.x + pos.y * int(u_field_size.x);
}

void main() {
	vec4 pixel        = vec4(0.0,0.0,0.0,1.0);
	ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);

    CellData curr = input_data[GetArrayId(pixel_coord)];

	CellData new_cell;
	new_cell.val = pixel_coord.x/u_field_size.x;
	next[GetArrayId(pixel_coord)] = new_cell;


    //imageStore(u_texture, pixel_coord, vec4(pixel_coord.x, pixel_coord.y, 1.0, 1.0));
    //imageStore(u_texture, ivec2(10, 10), vec4(pixel_coord.x, pixel_coord.y, 1.0, 1.0));
}
