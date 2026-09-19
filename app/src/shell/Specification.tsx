// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The Specification tab (IX-05): one element's general properties, owned
 * features, relationships and documentation, from the `element_detail` query.
 *
 * DECORATE, NEVER HIDE (ADR-0002). An unresolved type is a row with the type
 * marked unresolved, not a missing row, and the element's diagnostics are a
 * banner at the top (IX-10) — the element is shown as it is.
 *
 * OWNED ROWS ONLY, FOR NOW. Every feature row carries its `origin`, but OD-04
 * is undecided, so the table shows owned features and leaves inherited ones
 * out rather than mixing them in unmarked.
 *
 * Model text is rendered as text, never as markup (§8.1 rule 6).
 */

import type { ReactNode } from "react";

import type {
  ElementDetail,
  ElementFacet,
  ElementRef,
  FeatureRow,
  RelationshipRow,
} from "@/contract/element";
import type { ElementHandle } from "@/contract/element-id";
import { logLabel } from "@/model/element-handle";
import { featureKind, formatMultiplicity, keyword, relationshipLabel } from "@/model/element-label";
import { refName } from "@/model/view-label";

import { AnswerView } from "./AnswerView";
import { useAnswer } from "./services";

/** Props for `Specification`. */
export type SpecificationProps = Readonly<{ handle: ElementHandle }>;

/** The specification of the element `handle` addresses. */
export function Specification({ handle }: SpecificationProps): React.JSX.Element {
  const detail = useAnswer(
    (queries) => queries.elementDetail(handle),
    `element_detail:${logLabel(handle)}`,
  );
  return (
    <AnswerView query={detail} what="The specification">
      {(data) => <Detail detail={data} />}
    </AnswerView>
  );
}

function Detail({ detail }: Readonly<{ detail: ElementDetail }>): React.JSX.Element {
  const { summary } = detail;
  const owned = detail.ownedFeatures.filter((row) => row.origin.kind === "owned");
  return (
    <article
      aria-label={summary.name ?? "Unnamed element"}
      className="flex flex-col gap-4 p-3 text-ui"
    >
      <header>
        <p className="font-mono text-faint text-meta">«{keyword(summary.metaclass)}»</p>
        <h3 className="font-semibold text-name">{summary.name ?? "(unnamed)"}</h3>
        {summary.qualifiedName === null ? null : (
          <p className="font-mono text-faint text-meta">{summary.qualifiedName}</p>
        )}
      </header>
      {detail.diagnostics.length === 0 ? null : (
        <ul aria-label="Problems" className="rounded-wb border border-err p-2 text-err">
          {detail.diagnostics.map((diagnostic) => (
            <li key={`${diagnostic.code}:${diagnostic.span.start}`}>
              {diagnostic.severity}: {diagnostic.message}
            </li>
          ))}
        </ul>
      )}
      <Section title="General">
        <dl className="flex flex-col gap-1">
          <Property term="Name" value={summary.name ?? "—"} />
          <Property term="Kind" value={summary.metaclass} />
          <Property term="Owner" value={detail.owner === null ? "—" : <Ref to={detail.owner} />} />
          <FacetProperties facet={detail.facet} />
          <Property term="Visibility" value={detail.visibility ?? "—"} />
          <Property term="Short name" value={summary.shortName ?? "—"} />
          <Property term="File" value={detail.location?.file ?? "—"} />
        </dl>
      </Section>
      <Section title="Owned features">
        {owned.length === 0 ? <p className="text-muted">None.</p> : <FeatureTable rows={owned} />}
      </Section>
      <Section title="Relationships">
        {detail.relationships.length === 0 ? (
          <p className="text-muted">None.</p>
        ) : (
          <RelationshipList rows={detail.relationships} />
        )}
      </Section>
      <Section title="Documentation">
        {detail.documentation.length === 0 ? (
          <p className="text-muted">None.</p>
        ) : (
          detail.documentation.map((body, index) => (
            // Documentation bodies have no identity of their own; their order is theirs.
            // biome-ignore lint/suspicious/noArrayIndexKey: two identical doc bodies are legal, so the text cannot key them.
            <p key={index}>{body}</p>
          ))
        )}
      </Section>
    </article>
  );
}

function Section(props: Readonly<{ title: string; children: ReactNode }>): React.JSX.Element {
  return (
    <section aria-label={props.title}>
      <h4 className="mb-1 font-semibold text-meta text-muted uppercase">{props.title}</h4>
      {props.children}
    </section>
  );
}

function Property(props: Readonly<{ term: string; value: ReactNode }>): React.JSX.Element {
  return (
    <div className="flex gap-3">
      <dt className="w-24 shrink-0 text-muted">{props.term}</dt>
      <dd className="min-w-0">{props.value}</dd>
    </div>
  );
}

/** The properties only a type, or only a feature, has. */
function FacetProperties({ facet }: Readonly<{ facet: ElementFacet }>): ReactNode {
  if (facet.kind === "element") {
    return null;
  }
  const specializes = (
    <Property
      term="Specializes"
      value={
        facet.specializes.length === 0
          ? "—"
          : facet.specializes.map((row) => (
              <span key={logLabel(row.handle)} className="mr-2">
                {row.isImplied ? "(implicit) " : null}
                <Ref to={row.other} />
              </span>
            ))
      }
    />
  );
  if (facet.kind === "type") {
    return (
      <>
        {specializes}
        <Property term="Is abstract" value={String(facet.isAbstract)} />
      </>
    );
  }
  return (
    <>
      <Property term="Defined by" value={<Refs refs={facet.types} />} />
      <Property term="Multiplicity" value={formatMultiplicity(facet.multiplicity)} />
      <Property term="Is composite" value={String(facet.isComposite)} />
      <Property term="Redefines" value={<Refs refs={facet.redefines} />} />
      {specializes}
    </>
  );
}

function FeatureTable({ rows }: Readonly<{ rows: readonly FeatureRow[] }>): React.JSX.Element {
  return (
    <table className="w-full text-left">
      <thead className="text-muted text-meta">
        <tr>
          <th scope="col">Kind</th>
          <th scope="col">Name</th>
          <th scope="col">Type</th>
          <th scope="col">
            <abbr title="Multiplicity">[ ]</abbr>
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={logLabel(row.handle)}>
            <td className="font-mono text-faint">{featureKind(row.metaclass)}</td>
            <td>{row.name ?? "(unnamed)"}</td>
            <td>
              <Refs refs={row.types} />
            </td>
            <td className="font-mono">{formatMultiplicity(row.multiplicity)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function RelationshipList({
  rows,
}: Readonly<{ rows: readonly RelationshipRow[] }>): React.JSX.Element {
  return (
    <dl className="flex flex-col gap-1">
      {rows.map((row) => (
        <Property
          key={logLabel(row.handle)}
          term={relationshipLabel(row)}
          value={<Ref to={row.other} />}
        />
      ))}
    </dl>
  );
}

function Refs({ refs }: Readonly<{ refs: readonly ElementRef[] }>): ReactNode {
  if (refs.length === 0) {
    return "—";
  }
  return refs.map((ref, index) => (
    // biome-ignore lint/suspicious/noArrayIndexKey: an unresolved reference has no handle, and two may be written identically.
    <span key={index} className="mr-2">
      <Ref to={ref} />
    </span>
  ));
}

/** A reference, marked when it does not resolve (ADR-0002) rather than dropped. */
function Ref({ to }: Readonly<{ to: ElementRef }>): React.JSX.Element {
  if (to.kind === "resolved") {
    return <span>{refName(to)}</span>;
  }
  return (
    <span className="text-err underline decoration-wavy">
      {to.written}
      <span className="sr-only"> (unresolved)</span>
      <span aria-hidden="true"> ?</span>
    </span>
  );
}
