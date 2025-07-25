// MIT License

// Copyright (c) 2019-2021 bloc97
// All rights reserved.

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//!DESC Anime4K-v3.2-Deblur-Original-Luma
//!HOOK MAIN
//!BIND HOOKED
//!SAVE LINELUMA
//!COMPONENTS 1

float get_luma(vec4 rgba) {
	return dot(vec4(0.299, 0.587, 0.114, 0.0), rgba);
}

vec4 hook() {
	return vec4(get_luma(HOOKED_tex(HOOKED_pos)), 0.0, 0.0, 0.0);
}

//!DESC Anime4K-v3.2-Deblur-Original-Kernel-X
//!HOOK MAIN
//!BIND HOOKED
//!BIND LINELUMA
//!SAVE LUMAD
//!WIDTH MAIN.w 2 *
//!HEIGHT MAIN.h 2 *
//!COMPONENTS 2

vec4 hook() {
	vec2 d = HOOKED_pt;

	//[tl  t tr]
	//[ l  c  r]
	//[bl  b br]
	float l = LINELUMA_tex(HOOKED_pos + vec2(-d.x, 0.0)).x;
	float c = LINELUMA_tex(HOOKED_pos).x;
	float r = LINELUMA_tex(HOOKED_pos + vec2(d.x, 0.0)).x;

	//Horizontal Gradient
	//[-1  0  1]
	//[-2  0  2]
	//[-1  0  1]
	float xgrad = (-l + r);

	//Vertical Gradient
	//[-1 -2 -1]
	//[ 0  0  0]
	//[ 1  2  1]
	float ygrad = (l + c + c + r);

	//Computes the luminance's gradient
	return vec4(xgrad, ygrad, 0.0, 0.0);
}

//!DESC Anime4K-v3.2-Deblur-Original-Kernel-Y
//!HOOK MAIN
//!BIND LUMAD
//!SAVE LUMAD
//!WIDTH MAIN.w 2 *
//!HEIGHT MAIN.h 2 *
//!COMPONENTS 2

/* --------------------- SETTINGS --------------------- */

//Strength of edge refinement, good values are between 0.2 and 4
#define REFINE_STRENGTH 1.0

/* --- MODIFY THESE SETTINGS BELOW AT YOUR OWN RISK --- */

//Bias of the refinement function, good values are between 0 and 1
#define REFINE_BIAS 0.0

//Polynomial fit obtained by minimizing MSE error on image
#define P5 ( 11.68129591)
#define P4 (-42.46906057)
#define P3 ( 60.28286266)
#define P2 (-41.84451327)
#define P1 ( 14.05517353)
#define P0 (-1.081521930)

/* ----------------- END OF SETTINGS ----------------- */

float power_function(float x) {
	float x2 = x * x;
	float x3 = x2 * x;
	float x4 = x2 * x2;
	float x5 = x2 * x3;

	return P5*x5 + P4*x4 + P3*x3 + P2*x2 + P1*x + P0;
}

vec4 hook() {
	vec2 d = LUMAD_pt;

	//[tl  t tr]
	//[ l cc  r]
	//[bl  b br]
	float tx = LUMAD_tex(LUMAD_pos + vec2(0.0, -d.y)).x;
	float cx = LUMAD_tex(LUMAD_pos).x;
	float bx = LUMAD_tex(LUMAD_pos + vec2(0.0, d.y)).x;

	float ty = LUMAD_tex(LUMAD_pos + vec2(0.0, -d.y)).y;
	//float cy = LUMAD_tex(LUMAD_pos).y;
	float by = LUMAD_tex(LUMAD_pos + vec2(0.0, d.y)).y;

	//Horizontal Gradient
	//[-1  0  1]
	//[-2  0  2]
	//[-1  0  1]
	float xgrad = (tx + cx + cx + bx);

	//Vertical Gradient
	//[-1 -2 -1]
	//[ 0  0  0]
	//[ 1  2  1]
	float ygrad = (-ty + by);

	//Computes the luminance's gradient
	float sobel_norm = clamp(sqrt(xgrad * xgrad + ygrad * ygrad), 0.0, 1.0);

	float dval = clamp(power_function(clamp(sobel_norm, 0.0, 1.0)) * REFINE_STRENGTH + REFINE_BIAS, 0.0, 1.0);

	return vec4(sobel_norm, dval, 0.0, 0.0);
}

//!DESC Anime4K-v3.2-Deblur-Original-Kernel-X
//!HOOK MAIN
//!BIND LUMAD
//!SAVE LUMAMM
//!WIDTH MAIN.w 2 *
//!HEIGHT MAIN.h 2 *
//!COMPONENTS 2

vec4 hook() {
	vec2 d = LUMAD_pt;

	if (LUMAD_tex(LUMAD_pos).y < 0.1) {
		return vec4(0.0);
	}

	//[tl  t tr]
	//[ l  c  r]
	//[bl  b br]
	float l = LUMAD_tex(LUMAD_pos + vec2(-d.x, 0.0)).x;
	float c = LUMAD_tex(LUMAD_pos).x;
	float r = LUMAD_tex(LUMAD_pos + vec2(d.x, 0.0)).x;

	//Horizontal Gradient
	//[-1  0  1]
	//[-2  0  2]
	//[-1  0  1]
	float xgrad = (-l + r);

	//Vertical Gradient
	//[-1 -2 -1]
	//[ 0  0  0]
	//[ 1  2  1]
	float ygrad = (l + c + c + r);

	return vec4(xgrad, ygrad, 0.0, 0.0);
}

//!DESC Anime4K-v3.2-Deblur-Original-Kernel-Y
//!HOOK MAIN
//!BIND LUMAD
//!BIND LUMAMM
//!SAVE LUMAMM
//!WIDTH MAIN.w 2 *
//!HEIGHT MAIN.h 2 *
//!COMPONENTS 2

vec4 hook() {
	vec2 d = LUMAD_pt;

	if (LUMAD_tex(LUMAD_pos).y < 0.1) {
		return vec4(0.0);
	}

	//[tl  t tr]
	//[ l cc  r]
	//[bl  b br]
	float tx = LUMAMM_tex(LUMAMM_pos + vec2(0.0, -d.y)).x;
	float cx = LUMAMM_tex(LUMAMM_pos).x;
	float bx = LUMAMM_tex(LUMAMM_pos + vec2(0.0, d.y)).x;

	float ty = LUMAMM_tex(LUMAMM_pos + vec2(0.0, -d.y)).y;
	//float cy = LUMAMM_tex(LUMAMM_pos).y;
	float by = LUMAMM_tex(LUMAMM_pos + vec2(0.0, d.y)).y;

	//Horizontal Gradient
	//[-1  0  1]
	//[-2  0  2]
	//[-1  0  1]
	float xgrad = (tx + cx + cx + bx);

	//Vertical Gradient
	//[-1 -2 -1]
	//[ 0  0  0]
	//[ 1  2  1]
	float ygrad = (-ty + by);

	float norm = sqrt(xgrad * xgrad + ygrad * ygrad);
	if (norm <= 0.001) {
		xgrad = 0.0;
		ygrad = 0.0;
		norm = 1.0;
	}

	return vec4(xgrad/norm, ygrad/norm, 0.0, 0.0);
}

//!DESC Anime4K-v3.2-Deblur-Original-Apply
//!HOOK MAIN
//!BIND HOOKED
//!BIND LUMAD
//!BIND LUMAMM
//!WIDTH MAIN.w 2 *
//!HEIGHT MAIN.h 2 *
//!SAVE RESAMPLED

vec4 hook() {
	vec2 d = HOOKED_pt;

	float dval = LUMAD_tex(LUMAD_pos).y;
	if (dval < 0.1) {
		return HOOKED_tex(HOOKED_pos);
	}

	vec4 dc = LUMAMM_tex(LUMAD_pos);
	if (abs(dc.x + dc.y) <= 0.0001) {
		return HOOKED_tex(HOOKED_pos);
	}

	float xpos = -sign(dc.x);
	float ypos = -sign(dc.y);

	vec4 xval = HOOKED_tex(HOOKED_pos + vec2(d.x * xpos, 0.0));
	vec4 yval = HOOKED_tex(HOOKED_pos + vec2(0.0, d.y * ypos));

	float xyratio = abs(dc.x) / (abs(dc.x) + abs(dc.y));

	vec4 avg = xyratio * xval + (1.0 - xyratio) * yval;

	return avg * dval + HOOKED_tex(HOOKED_pos) * (1.0 - dval);
}

//!DESC Anime4K-v3.2-Deblur-Original-Resample
//!HOOK MAIN
//!BIND HOOKED
//!BIND RESAMPLED

vec4 hook() {
	return RESAMPLED_tex(HOOKED_pos);
}
