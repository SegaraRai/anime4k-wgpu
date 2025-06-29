import db from "mime-db";

export const ALL_VIDEO_MEDIA_TYPES: readonly string[] = Object.keys(db)
  .filter((key) => key.startsWith("video/"))
  .sort();
