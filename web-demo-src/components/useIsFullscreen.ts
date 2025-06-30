import { useLayoutEffect, useState } from "preact/hooks";

export function useIsFullscreen(): boolean {
  const [isFullscreen, setIsFullscreen] = useState(false);
  useLayoutEffect(() => {
    const handleFullscreenChange = (): void => {
      setIsFullscreen(document.fullscreenElement !== null);
    };

    document.addEventListener("fullscreenchange", handleFullscreenChange);
    handleFullscreenChange();

    return () => {
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, []);

  return isFullscreen;
}
