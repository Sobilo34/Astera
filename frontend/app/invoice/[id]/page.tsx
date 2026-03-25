'use client';

import { useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import Link from 'next/link';
import { getInvoice, getInvoiceMetadata } from '@/lib/contracts';
import { formatUSDC, formatDate, daysUntil } from '@/lib/stellar';
import type { Invoice, InvoiceMetadata } from '@/lib/types';
import { useStore } from '@/lib/store';

export default function InvoiceDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { wallet } = useStore();
  const [invoice, setInvoice] = useState<Invoice | null>(null);
  const [metadata, setMetadata] = useState<InvoiceMetadata | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadInvoice();
  }, [id]);

  async function loadInvoice() {
    try {
      const numId = parseInt(id, 10);
      const [inv, meta] = await Promise.all([getInvoice(numId), getInvoiceMetadata(numId)]);
      setInvoice(inv);
      setMetadata(meta);
    } catch (e) {
      setError('Invoice not found or contracts not deployed.');
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen pt-24 px-6">
        <div className="max-w-2xl mx-auto space-y-4">
          {[1, 2, 3].map((n) => (
            <div key={n} className="h-24 bg-brand-card rounded-2xl animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  if (error || !invoice || !metadata) {
    return (
      <div className="min-h-screen pt-24 px-6 flex flex-col items-center justify-center text-center">
        <p className="text-red-400 mb-4">{error ?? 'Invoice not found.'}</p>
        <Link href="/dashboard" className="text-brand-gold hover:underline text-sm">
          Back to Dashboard
        </Link>
      </div>
    );
  }

  const days = daysUntil(metadata.dueDate);
  const isOwner = wallet.address === invoice.owner;

  const timeline = [
    { label: 'Created', ts: invoice.createdAt, done: true },
    { label: 'Funded', ts: invoice.fundedAt, done: metadata.status !== 'Pending' },
    { label: 'Paid', ts: invoice.paidAt, done: metadata.status === 'Paid' },
  ];

  return (
    <div className="min-h-screen pt-24 pb-16 px-6">
      <div className="max-w-2xl mx-auto">
        {/* Back */}
        <Link
          href="/dashboard"
          className="text-brand-muted hover:text-white text-sm mb-6 inline-flex items-center gap-2 transition-colors"
        >
          ← Back to Dashboard
        </Link>

        {/* Header */}
        <div className="p-6 bg-brand-card border border-brand-border rounded-2xl mb-6">
          {metadata.image ? (
            <div className="mb-6 rounded-xl overflow-hidden border border-brand-border bg-brand-dark">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img src={metadata.image} alt="" className="w-full h-40 object-cover" />
            </div>
          ) : null}
          <div className="flex items-start justify-between mb-6 gap-4">
            <div className="min-w-0">
              <p className="text-xs text-brand-muted mb-1">
                {metadata.symbol} · Invoice #{invoice.id}
              </p>
              <h1 className="text-2xl font-bold">{metadata.name}</h1>
              <p className="text-brand-muted mt-1">{metadata.debtor}</p>
            </div>
            <span
              className={`text-sm font-medium px-3 py-1.5 rounded-full flex-shrink-0 badge-${metadata.status.toLowerCase()}`}
            >
              {metadata.status}
            </span>
          </div>

          <div className="text-4xl font-bold gradient-text mb-6">{formatUSDC(metadata.amount)}</div>

          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <p className="text-brand-muted mb-1">Due Date</p>
              <p className="font-medium">{formatDate(metadata.dueDate)}</p>
            </div>
            <div>
              <p className="text-brand-muted mb-1">Time Remaining</p>
              <p
                className={`font-medium ${
                  days < 0 ? 'text-red-400' : days <= 7 ? 'text-yellow-400' : 'text-white'
                }`}
              >
                {days < 0 ? `${Math.abs(days)} days overdue` : `${days} days`}
              </p>
            </div>
            <div className="col-span-2">
              <p className="text-brand-muted mb-1">Owner</p>
              <p className="font-mono text-xs text-white break-all">{invoice.owner}</p>
            </div>
            {metadata.description && (
              <div className="col-span-2">
                <p className="text-brand-muted mb-1">Description</p>
                <p className="text-sm">{metadata.description}</p>
              </div>
            )}
          </div>
        </div>

        {/* Timeline */}
        <div className="p-6 bg-brand-card border border-brand-border rounded-2xl mb-6">
          <h2 className="text-lg font-semibold mb-6">Timeline</h2>
          <div className="space-y-4">
            {timeline.map((step, i) => (
              <div key={step.label} className="flex items-center gap-4">
                <div
                  className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 text-xs font-bold ${
                    step.done ? 'bg-brand-gold text-brand-dark' : 'bg-brand-border text-brand-muted'
                  }`}
                >
                  {step.done ? '✓' : i + 1}
                </div>
                <div className="flex-1 flex justify-between">
                  <span className={step.done ? 'text-white font-medium' : 'text-brand-muted'}>
                    {step.label}
                  </span>
                  {step.done && step.ts > 0 && (
                    <span className="text-brand-muted text-sm">{formatDate(step.ts)}</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Actions */}
        {isOwner && metadata.status === 'Pending' && (
          <div className="p-4 bg-brand-gold/10 border border-brand-gold/20 rounded-xl text-sm text-brand-muted">
            Your invoice is pending review. Once approved, the pool will fund it and USDC will be
            sent to your wallet.
          </div>
        )}
      </div>
    </div>
  );
}
