import type { Scene } from "~/lib/vtt";
import { activeAt, progressAt, shares } from "./rail";

/**
 * The video's scenes in order, each as wide as its share of the running time:
 * the one playing is named in ink and filled as it plays, and picking one goes
 * to its start. On a phone the playing scene is named under the segments.
 */
export function SceneRail({
  scenes,
  time,
  onPick,
}: {
  scenes: Scene[];
  time: number;
  onPick: (index: number) => void;
}) {
  const active = activeAt(scenes, time);
  const widths = shares(scenes);
  return (
    <>
      <ol aria-label="Scenes of the video" className="mt-3 flex gap-1.5">
        {scenes.map((scene, i) => {
          const current = i === active;
          const fill = i < active ? 1 : current ? progressAt(scenes, time, i) : 0;
          return (
            <li key={scene.name} style={{ flex: `${widths[i]} 1 0` }} className="min-w-0">
              <button
                type="button"
                onClick={() => {
                  onPick(i);
                }}
                aria-current={current ? "step" : undefined}
                aria-label={`${scene.name}: go to this scene`}
                className="group block w-full cursor-pointer pt-1 pb-1.5 text-left"
              >
                <span className="block h-[3px] overflow-hidden rounded-full bg-line">
                  <span
                    className="block h-full origin-left rounded-full bg-accent"
                    style={{ transform: `scaleX(${fill})` }}
                  />
                </span>
                <span
                  className={`mt-2 hidden truncate text-[14px] leading-none sm:block ${
                    current ? "font-medium text-text" : "text-muted group-hover:text-text"
                  }`}
                >
                  {scene.name}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
      {/* on a phone the segments are too narrow to name: the playing one, under them */}
      <p aria-hidden="true" className="mt-1 text-[14px] font-medium sm:hidden">
        {scenes[active]?.name}
      </p>
    </>
  );
}
