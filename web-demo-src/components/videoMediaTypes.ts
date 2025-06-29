import { ALL_VIDEO_MEDIA_TYPES } from "./videoMediaTypes.macro" with { type: "macro" };

// It seems that the macro parser becomes unhappy unless it is temporarily stored in a variable
const temp = ALL_VIDEO_MEDIA_TYPES;

export function getBrowserSupportedVideoMediaTypes(): readonly string[] {
  // Filter out media types that are not supported by the browser
  const video = document.createElement("video");
  return temp.filter((type) => video.canPlayType(type) !== "");
}
