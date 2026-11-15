import React, { useState } from 'react';
import { CheckCircle2, Loader2, ShieldAlert, Wrench } from 'lucide-react';
import {
  Alert,
  AlertDescription,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

interface NvramPatchCardProps {
  defaultInputPath?: string;
  patching: boolean;
  patchSuccess: boolean;
  error: string | null;
  lastOutputFile?: string | null;
  onExecutePatch: (params: {
    inputPath: string;
    outputPath: string;
    nvramOffset: string;
    patchHex: string;
  }) => Promise<unknown>;
}

export const NvramPatchCard: React.FC<NvramPatchCardProps> = ({
  defaultInputPath,
  patching,
  patchSuccess,
  error,
  lastOutputFile,
  onExecutePatch,
}) => {
  const { t } = useTranslation();
  const [inputPath, setInputPath] = useState(
    defaultInputPath || 'nor_dump.bin',
  );
  const [outputPath, setOutputPath] = useState('nor_dump.patched.bin');
  const [nvramOffset, setNvramOffset] = useState('0x0');
  const [patchHex, setPatchHex] = useState('FFFFFFFF');

  return (
    <Card data-testid="nvram-cow-patch-card">
      <CardHeader>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Wrench className="w-4 h-4 text-cyan-400" />
            <CardTitle>{t('nvram.patchTitle')}</CardTitle>
          </div>
          <Badge variant="outline" className="font-mono text-[10px] border-amber-500/40 text-amber-300">
            CoW + IPL E05 RO
          </Badge>
        </div>
        <CardDescription>{t('nvram.patchDesc')}</CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
          <div className="space-y-1">
            <label htmlFor="nvram-patch-input" className="text-[11px] font-mono text-slate-400">
              {t('nvram.patchInputLabel')}
            </label>
            <Input
              id="nvram-patch-input"
              value={inputPath}
              onChange={e => setInputPath(e.target.value)}
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="nvram-patch-output" className="text-[11px] font-mono text-slate-400">
              {t('nvram.patchOutputLabel')}
            </label>
            <Input
              id="nvram-patch-output"
              value={outputPath}
              onChange={e => setOutputPath(e.target.value)}
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="nvram-patch-offset" className="text-[11px] font-mono text-slate-400">
              {t('nvram.patchOffsetLabel')}
            </label>
            <Input
              id="nvram-patch-offset"
              value={nvramOffset}
              onChange={e => setNvramOffset(e.target.value)}
              placeholder="0x0"
            />
          </div>

          <div className="space-y-1">
            <label htmlFor="nvram-patch-hex" className="text-[11px] font-mono text-slate-400">
              {t('nvram.patchHexLabel')}
            </label>
            <Input
              id="nvram-patch-hex"
              value={patchHex}
              onChange={e => setPatchHex(e.target.value)}
              placeholder="FFFFFFFF"
            />
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button
            type="button"
            disabled={patching}
            onClick={() =>
              void onExecutePatch({
                inputPath,
                outputPath,
                nvramOffset,
                patchHex,
              })
            }
            data-testid="nvram-cow-patch-btn"
          >
            {patching && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
            {t('nvram.patchBtn')}
          </Button>

          {patchSuccess && (
            <span className="inline-flex items-center gap-1.5 text-xs font-mono text-emerald-400">
              <CheckCircle2 className="w-4 h-4" />
              {t('nvram.patchSuccess')}
              {lastOutputFile ? ` (${lastOutputFile})` : ''}
            </span>
          )}
        </div>

        {error && (
          <Alert variant="destructive" className="py-2">
            <ShieldAlert className="h-4 w-4" />
            <AlertDescription className="text-xs font-mono">{error}</AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
};
