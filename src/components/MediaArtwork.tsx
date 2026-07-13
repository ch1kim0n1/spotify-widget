import { useState } from "react";
import { WaveIcon } from "./Icons";

export interface MediaArtworkProps {
  url: string | null;
  title: string;
  stale?: boolean;
}

export function MediaArtwork({ url, title, stale = false }: MediaArtworkProps) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);

  return (
    <div className="artwork" data-stale={stale || undefined}>
      {url && url !== failedUrl ? (
        <img src={url} alt="" draggable={false} onError={() => setFailedUrl(url)} />
      ) : (
        <div className="artwork__fallback" aria-hidden="true">
          <WaveIcon />
        </div>
      )}
      {stale && <span className="artwork__badge">Last known</span>}
      <span className="sr-only">{title ? `Artwork for ${title}` : "No artwork available"}</span>
    </div>
  );
}
