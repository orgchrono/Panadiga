import React from 'react';

export interface BridgeNoticeBannerProps {
  readonly role: 'note' | 'alert';
  readonly toneClass: string;
  readonly titleClass: string;
  readonly bodyClass: string;
  readonly title: string;
  readonly body: string;
}

export const BridgeNoticeBanner: React.FC<BridgeNoticeBannerProps> = ({
  role,
  toneClass,
  titleClass,
  bodyClass,
  title,
  body,
}) => (
  <div
    role={role}
    aria-label={title}
    className={`rounded-lg border px-3 py-2 space-y-0.5 ${toneClass}`}
  >
    <div className={`font-semibold ${titleClass}`}>{title}</div>
    <p className={`leading-relaxed ${bodyClass}`}>{body}</p>
  </div>
);
