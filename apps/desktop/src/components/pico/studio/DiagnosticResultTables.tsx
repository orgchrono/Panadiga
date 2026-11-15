import React from 'react';
import type { PicoDiagnosticResult } from '@/models';

export interface DiagnosticResultTablesProps {
  result: PicoDiagnosticResult;
}

export const DiagnosticResultTables: React.FC<DiagnosticResultTablesProps> = ({ result }) => (
  <>
    {result.pincheck_rows.length > 0 && (
      <div className="overflow-x-auto rounded-lg border border-slate-800">
        <table className="w-full text-[11px]">
          <thead className="bg-slate-900 text-slate-400">
            <tr>
              <th className="text-left p-2">Pino</th>
              <th className="text-left p-2">Hi-Z</th>
              <th className="text-left p-2">Pull-Down</th>
              <th className="text-left p-2">Pull-Up</th>
              <th className="text-left p-2">Estado Elétrico</th>
            </tr>
          </thead>
          <tbody>
            {result.pincheck_rows.map(row => (
              <tr key={row.pin_label} className="border-t border-slate-800/80 text-slate-200">
                <td className="p-2 font-bold text-cyan-300">{row.pin_label}</td>
                <td className="p-2">{row.hiz}</td>
                <td className="p-2">{row.pull_down}</td>
                <td className="p-2">{row.pull_up}</td>
                <td className="p-2 font-sans text-slate-300">{row.state}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )}

    {result.transition_rows.length > 0 && (
      <div className="overflow-x-auto rounded-lg border border-slate-800">
        <table className="w-full text-[11px]">
          <thead className="bg-slate-900 text-slate-400">
            <tr>
              <th className="text-left p-2">Sinal</th>
              <th className="text-left p-2">GPIO</th>
              <th className="text-left p-2">Nível Final</th>
              <th className="text-left p-2">Transições</th>
            </tr>
          </thead>
          <tbody>
            {result.transition_rows.map(row => (
              <tr key={`${row.signal}-${row.gpio}`} className="border-t border-slate-800/80">
                <td className="p-2 font-bold text-cyan-300">{row.signal}</td>
                <td className="p-2 text-slate-400">{row.gpio}</td>
                <td className="p-2 text-slate-200">{row.final_level ?? '-'}</td>
                <td className="p-2 text-emerald-400 font-bold">{row.transitions}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )}

    {result.dump_rows.length > 0 && (
      <div className="overflow-x-auto rounded-lg border border-slate-800">
        <table className="w-full text-[11px]">
          <thead className="bg-slate-900 text-slate-400">
            <tr>
              <th className="text-left p-2">Endereço</th>
              <th className="text-left p-2">Palavras 32-bit</th>
              <th className="text-left p-2">ASCII LE</th>
            </tr>
          </thead>
          <tbody>
            {result.dump_rows.map(row => (
              <tr key={row.address} className="border-t border-slate-800/80">
                <td className="p-2 font-bold text-cyan-400">{row.address}</td>
                <td className="p-2 text-slate-200">{row.words.join(' ')}</td>
                <td className="p-2 text-amber-300">{row.ascii}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )}
  </>
);
