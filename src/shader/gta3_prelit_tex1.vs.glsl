#version 140

in vec3 pos;
in vec4 color;
in vec2 uv0;
out vec2 v_uv0;
out vec4 v_color;

uniform mat4 model_view_proj;

void main() {
    v_color = color;
    v_uv0 = vec2(uv0.x, 1.0 - uv0.y);
    gl_Position = model_view_proj * vec4(pos.x, pos.z, pos.y, 1.0);// + vec4(0.0, 0.0, 0.0, 0.0);
}
