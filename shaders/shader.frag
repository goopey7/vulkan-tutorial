#version 450

// input color and texture coord from vertex shader
layout(location=0) in vec3 fragColor;
layout(location=1) in vec2 fragTexCoord;

// uniform binding for sampler
layout(binding=1) uniform sampler2D texSampler;

// push constant
layout(push_constant) uniform PushConstants
{
	layout(offset = 64) float opacity;
} pcs;

// create variable for framebuffer (we have one so index 0)
layout(location=0) out vec4 outColor;

// called for every fragment (which was output from the vertex shader)
void main()
{
	outColor = vec4(texture(texSampler, fragTexCoord).rgb, pcs.opacity);
}
