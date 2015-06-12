#version 140

in vec4 v_color;
in vec2 v_uv0;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, v_uv0) * v_color;
}