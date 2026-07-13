import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function IconBase({ children, ...props }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" {...props}>
      {children}
    </svg>
  );
}

export function PreviousIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path d="M6.5 5v14M18.5 6.2 9.2 12l9.3 5.8V6.2Z" />
    </IconBase>
  );
}

export function NextIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path d="M17.5 5v14M5.5 6.2l9.3 5.8-9.3 5.8V6.2Z" />
    </IconBase>
  );
}

export function PlayIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path d="m8 5 11 7-11 7V5Z" />
    </IconBase>
  );
}

export function PauseIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path d="M7 5h3.7v14H7V5Zm6.3 0H17v14h-3.7V5Z" />
    </IconBase>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path d="m7 7 10 10M17 7 7 17" fill="none" stroke="currentColor" strokeWidth="1.8" />
    </IconBase>
  );
}

export function WaveIcon(props: IconProps) {
  return (
    <IconBase {...props}>
      <path
        d="M3 12h3l1.5-4.5L10.3 17l2.5-7.5 1.8 5.2L16 12h5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </IconBase>
  );
}
