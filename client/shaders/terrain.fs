#version 330

in vec3 fragPosition;
in vec2 fragTexCoord;
flat in vec4 fragColor;

// Fog
uniform vec4 colDiffuse;
uniform vec3 cameraPosition;
uniform vec4 fogColor;
uniform float fogNear;
uniform float fogFar;

// Lighting
uniform vec3 sunDirection;
uniform vec3 sunColor;
uniform float sunIntensity;
uniform float ambientStrength;

out vec4 finalColor;

void main()
{
  vec4 baseColor = fragColor;

  // Ambient
  vec3 ambient = ambientStrength * baseColor.rgb;

  // Diffuse (Lambert)
  vec3 norm = normalize(cross(dFdx(fragPosition), dFdy(fragPosition)));
  if (norm.y < 0.0) norm = -norm;
  float diff = max(dot(norm, normalize(sunDirection)), 0.0);
  vec3 diffuse = diff * sunIntensity * sunColor * baseColor.rgb;

  // Combine and clamp
  vec3 litColor = min(ambient + diffuse, vec3(1.0));
  vec4 texelColor = vec4(litColor, baseColor.a);

  // Fog
  float dist = length(cameraPosition - fragPosition);
  float fogFactor = clamp((dist - fogNear) / (fogFar - fogNear), 0.0, 1.0);
  
  finalColor = mix(texelColor, fogColor, fogFactor);
}
