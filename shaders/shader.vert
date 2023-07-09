#version 450

// input from vertex buffer
layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;

// output color
layout(location = 0) out vec3 fragColor;

// gets invoked for each vertex
void main()
{
	gl_Position = vec4(inPosition, 0.0, 1.0);
	fragColor = inColor;
}
