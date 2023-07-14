#version 450

// input vertex attricutes
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

// output color and texture coord
layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragTexCoord;

// Uniform Buffer - Model View Projection Matrix
layout(binding = 0) uniform UniformBufferObject
{
	mat4 view;
	mat4 proj;
} ubo;

// Push Constant
layout(push_constant) uniform PushConstants
{
	mat4 model;
} pcs;

// gets invoked for each vertex
void main()
{
	gl_Position = ubo.proj * ubo.view * pcs.model * vec4(inPosition, 1.0);
	fragColor = inColor;
	fragTexCoord = inTexCoord;
}
