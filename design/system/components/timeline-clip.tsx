import type { Child, Markup } from "@design/jsx/jsx-runtime";

import type { Waveform } from "./waveform-samples.ts";

const CLIP = {
  text: { fill: "bg-[#7251a3]", border: "border-[#8261b3]" },
  video: { fill: "bg-[#294d75]", border: "border-[#395d85]" },
  audio: { fill: "bg-[#24656b]", border: "border-[#34757b]" },
} as const;

/// A 64px clip body with a 1px border. Children are placed from the clip's outer edge;
/// `absolute` offsets in CSS start inside the border, hence the 1px less in each label.
function ClipFrame(props: {
  kind: keyof typeof CLIP;
  width: number;
  selected?: boolean;
  locked?: boolean;
  children?: Child;
}): Markup {
  const colors = CLIP[props.kind];
  return (
    <div
      class={`relative h-16 overflow-hidden rounded-md border ${colors.fill} ${
        props.selected ? "border-accent" : colors.border
      }`}
      style={`width:${props.width}px`}
    >
      {props.children}
      {props.locked && <div class="hatch-lock-clip absolute inset-0" />}
    </div>
  );
}

/// The clip name, 12px semibold, with its baseline 20px (top) or 52px (bottom) below the clip's top.
function ClipName(props: { name: string; bottom?: boolean }): Markup {
  // SF Pro's baseline sits 0.8605 × font size below the top of a `leading-none` box.
  const top = (props.bottom ? 52 : 20) - 12 * 0.8605 - 1;
  return (
    <span
      class="absolute left-[7px] text-xs leading-none font-semibold whitespace-nowrap text-text"
      style={`top:${top}px`}
    >
      {props.name}
    </span>
  );
}

function ClipWaveform(props: { waveform: Waveform; width: number; height?: number }): Markup {
  const { waveform } = props;
  const height = props.height ?? 64;
  return (
    <svg
      aria-hidden="true"
      class="absolute -top-px -left-px"
      width={waveform.width}
      height={height}
      viewBox={`0 0 ${waveform.width} 64`}
      preserveAspectRatio="none"
    >
      <path d={waveform.d} stroke="#69c5cf" stroke-opacity="0.82" stroke-width="2" fill="none" />
    </svg>
  );
}

/// A miniature source frame, drawn in a portrait 9:16 box and cropped to fill any
/// target size. Variants 0-2 are moments of one video so a filmstrip reads as motion;
/// variant 3 is a still photograph for image clips.
export function FrameThumb(
  props: { width: number; height: number; variant?: number; class?: string },
): Markup {
  const variant = props.variant ?? 0;
  return (
    <svg
      aria-hidden="true"
      class={props.class}
      width={props.width}
      height={props.height}
      viewBox="0 0 90 160"
      preserveAspectRatio="xMidYMid slice"
    >
      <rect width="90" height="160" fill="#0c2b21" />
      {variant === 3
        ? (
          <g>
            <rect width="90" height="160" fill="#123a5c" />
            <circle cx="66" cy="34" r="12" fill="#f7d774" />
            <path d="M0 118L28 74L52 112L70 88L90 116V160H0Z" fill="#0b2438" />
            <path d="M0 132L20 104L44 136L64 112L90 140V160H0Z" fill="#081a29" />
          </g>
        )
        : (
          <g>
            <rect x="8" y="10" width="10" height="10" rx="2" fill="#34d399" />
            <rect x="22" y="13" width="22" height="4" rx="2" fill="#e8f6ef" />
            {variant === 0 && (
              <g>
                <rect x="8" y="40" width="62" height="7" rx="2" fill="#f2fbf7" />
                <rect x="8" y="51" width="46" height="7" rx="2" fill="#6ee7b7" />
                <rect x="8" y="64" width="52" height="3" rx="1.5" fill="#7fa896" />
                <rect x="8" y="84" width="74" height="46" rx="3" fill="#f4f6f8" />
                <rect x="13" y="90" width="30" height="4" rx="2" fill="#c3ccd4" />
                <rect x="13" y="100" width="64" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="13" y="108" width="58" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="13" y="116" width="64" height="3" rx="1.5" fill="#dde3e8" />
              </g>
            )}
            {variant === 1 && (
              <g>
                <rect x="6" y="30" width="78" height="100" rx="3" fill="#f4f6f8" />
                <rect x="11" y="36" width="34" height="5" rx="2" fill="#b9c3cc" />
                <rect x="11" y="48" width="68" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="11" y="56" width="62" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="11" y="64" width="68" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="11" y="72" width="56" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="11" y="86" width="24" height="8" rx="4" fill="#d9f2e5" />
                <rect x="11" y="100" width="68" height="3" rx="1.5" fill="#dde3e8" />
                <rect x="11" y="108" width="60" height="3" rx="1.5" fill="#dde3e8" />
              </g>
            )}
            {variant === 2 && (
              <g>
                <rect x="37" y="58" width="16" height="16" rx="3" fill="#34d399" />
                <rect x="25" y="82" width="40" height="5" rx="2" fill="#f2fbf7" />
                <rect x="14" y="100" width="20" height="8" rx="4" fill="#123f31" />
                <rect x="36" y="100" width="20" height="8" rx="4" fill="#123f31" />
                <rect x="58" y="100" width="18" height="8" rx="4" fill="#123f31" />
              </g>
            )}
          </g>
        )}
    </svg>
  );
}

/// Repeating source frames across a clip body, one cell per 36px of width.
function Filmstrip(props: { width: number }): Markup {
  const cell = 36;
  const count = Math.ceil(props.width / cell);
  return (
    <div class="absolute -top-px -left-px flex" style={`width:${props.width}px;height:64px`}>
      {Array.from({ length: count }, (_, index) => (
        <div class="relative shrink-0" style={`width:${cell}px;height:64px`}>
          <FrameThumb width={cell} height={64} variant={index % 3} />
          {index > 0 && <div class="absolute top-0 left-0 h-full w-px bg-black/40" />}
        </div>
      ))}
    </div>
  );
}

export function TextClip(props: { width: number; name: string; selected?: boolean }): Markup {
  return (
    <ClipFrame kind="text" width={props.width} selected={props.selected}>
      <ClipName name={props.name} />
    </ClipFrame>
  );
}

/// A video clip. With `filmstrip`, the body shows repeating source frames and the
/// name moves to a top scrim; an optional thin waveform band keeps the audio legible.
/// Without it, the legacy body is the audio waveform under the name.
export function VideoClip(
  props: {
    width: number;
    name: string;
    waveform?: Waveform;
    selected?: boolean;
    locked?: boolean;
    labelTop?: boolean; // the name at the top instead of the bottom
    filmstrip?: boolean;
  },
): Markup {
  if (props.filmstrip) {
    return (
      <ClipFrame kind="video" width={props.width} selected={props.selected} locked={props.locked}>
        <Filmstrip width={props.width} />
        <div
          class="absolute inset-x-0 top-0 h-7"
          style="background:linear-gradient(to bottom,rgba(5,8,10,.78),rgba(5,8,10,0))"
        />
        {props.waveform && (
          <div class="absolute inset-x-0 bottom-0 h-4 overflow-hidden bg-black/45">
            <ClipWaveform waveform={props.waveform} width={props.width} height={16} />
          </div>
        )}
        <ClipName name={props.name} />
      </ClipFrame>
    );
  }
  return (
    <ClipFrame kind="video" width={props.width} selected={props.selected} locked={props.locked}>
      {props.waveform && <ClipWaveform waveform={props.waveform} width={props.width} />}
      <ClipName name={props.name} bottom={!props.labelTop} />
    </ClipFrame>
  );
}

/// An image clip: the still itself fills the body, name on a top scrim.
export function ImageClip(
  props: { width: number; name: string; selected?: boolean; locked?: boolean },
): Markup {
  return (
    <ClipFrame kind="video" width={props.width} selected={props.selected} locked={props.locked}>
      <FrameThumb width={props.width} height={64} variant={3} class="absolute -top-px -left-px" />
      <div
        class="absolute inset-x-0 top-0 h-7"
        style="background:linear-gradient(to bottom,rgba(5,8,10,.78),rgba(5,8,10,0))"
      />
      <ClipName name={props.name} />
    </ClipFrame>
  );
}

/// An audio clip: the waveform, the name at the top and an "Audio" label at the bottom.
export function AudioClip(props: { width: number; name: string; waveform: Waveform; selected?: boolean }): Markup {
  const labelTop = 52 - 12 * 0.8605 - 1;
  return (
    <ClipFrame kind="audio" width={props.width} selected={props.selected}>
      <ClipWaveform waveform={props.waveform} width={props.width} />
      <ClipName name={props.name} />
      <span class="absolute left-[7px] font-mono text-xs leading-none text-[#c8d8e8]" style={`top:${labelTop}px`}>
        Audio
      </span>
    </ClipFrame>
  );
}

/// An existing audio clip seen beside the one being placed, with a plain 11-12px label.
export function NeighborClip(
  props: { width: number; name: string; size: number; left: number; baseline: number },
): Markup {
  return (
    <div
      class="relative h-16 overflow-hidden rounded-md border border-[#34757b] bg-[#24656b]"
      style={`width:${props.width}px`}
    >
      <span
        class="absolute leading-none whitespace-nowrap text-[#c8d8e8]"
        style={`left:${props.left - 1}px;top:${props.baseline - props.size * 0.8605 - 1}px;font-size:${props.size}px`}
      >
        {props.name}
      </span>
    </div>
  );
}
