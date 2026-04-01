import { useEffect, useRef } from "react";

export function PixelBackground() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    let disposed = false;
    let background:
      | {
          stop: () => void;
        }
      | undefined;

    async function mount() {
      const canvas = canvasRef.current;

      if (!canvas) {
        return;
      }

      const wasm = await import("@/wasm/pixel-bg_pkg/pixel_bg_wasm.js");

      if (disposed) {
        return;
      }

      await wasm.default();

      if (disposed) {
        return;
      }

      background = new wasm.PixelBackground(canvas);
    }

    void mount();

    return () => {
      disposed = true;
      background?.stop();
    };
  }, []);

  return (
    <div aria-hidden="true" className="absolute inset-0 overflow-hidden pointer-events-none">
      <canvas
        className="absolute inset-0 h-full w-full"
        ref={canvasRef}
      />
    </div>
  );
}
