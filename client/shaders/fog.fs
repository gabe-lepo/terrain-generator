#version 330

in vec3 fragPosition;
in vec2 fragTexCoord;
in vec4 fragColor;
in vec3 fragNormal;

uniform vec4 colDiffuse;
uniform vec3 cameraPosition;
uniform vec4 fogColor;
uniform float fogNear;
uniform float fogFar;

out vec4 finalColor;

void main()
{
  // Base color from vertex colors
  vec4 texelColor = fragColor;

  // Calculate distance from camera to fragment
  float distance = length(cameraPosition - fragPosition);

  // Calculate fog factor (0.0 = no fog, 1.0 = full fog)
  float fogFactor = clamp((distance - fogNear) / (fogFar - fogNear), 0.0, 1.0);

  // Mix fragment color with fog color
  finalColor = mix(texelColor, fogColor, fogFactor);
}
