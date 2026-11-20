import React from 'react';
import { Badge, CardDescription, CardHeader, CardTitle, type BadgeProps } from '@/components/ui';

export interface PipelineCardHeaderProps {
  readonly icon: React.ReactNode;
  readonly title: string;
  readonly subtitle: string;
  readonly badge: string;
  readonly badgeVariant?: BadgeProps['variant'];
}

export const PipelineCardHeader: React.FC<PipelineCardHeaderProps> = ({
  icon,
  title,
  subtitle,
  badge,
  badgeVariant = 'warning',
}) => (
  <CardHeader className="pb-2">
    <div className="flex flex-wrap items-center justify-between gap-2">
      <div className="space-y-1">
        <CardTitle className="text-base text-slate-100 flex items-center gap-2 font-mono">
          {icon}
          <span>{title}</span>
        </CardTitle>
        <CardDescription className="text-xs text-slate-400">{subtitle}</CardDescription>
      </div>
      <Badge variant={badgeVariant}>{badge}</Badge>
    </div>
  </CardHeader>
);
