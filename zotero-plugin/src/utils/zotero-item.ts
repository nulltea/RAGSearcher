/**
 * Helpers for extracting metadata from Zotero items.
 */

const RAG_ID_PREFIX = "RAG-ID: ";

/** Extract paper metadata from a Zotero item */
export function getItemMetadata(item: Zotero.Item): {
  title: string;
  authors: string;
  source: string | undefined;
  paperType: string;
} {
  const title = item.getField("title") as string;

  const creators = item.getCreators();
  const authors = creators
    .filter((c: any) => c.creatorType === "author")
    .map((c: any) => [c.firstName, c.lastName].filter(Boolean).join(" "))
    .join(", ");

  // Try DOI first, then URL
  const doi = item.getField("DOI") as string;
  const url = item.getField("url") as string;
  const source = doi ? `https://doi.org/${doi}` : url || undefined;

  // Map Zotero item types to paper types
  const itemType = item.itemType;
  const typeMap: Record<string, string> = {
    journalArticle: "research_paper",
    conferencePaper: "research_paper",
    preprint: "research_paper",
    report: "technical_report",
    bookSection: "book_chapter",
    blogPost: "blog_post",
    webpage: "article",
  };
  const paperType = typeMap[itemType] || "research_paper";

  return { title, authors, source, paperType };
}

/** Get the PDF attachment file path from a Zotero item */
export async function getPdfPath(item: Zotero.Item): Promise<string | null> {
  // Check if the item itself is a PDF attachment
  if (item.isAttachment() && item.attachmentContentType === "application/pdf") {
    return (await item.getFilePathAsync()) as string | null;
  }

  // Find PDF attachment among children
  const attachmentIDs = item.getAttachments();
  for (const id of attachmentIDs) {
    const attachment = Zotero.Items.get(id);
    if (attachment && attachment.attachmentContentType === "application/pdf") {
      return (await attachment.getFilePathAsync()) as string | null;
    }
  }

  return null;
}

/** Get the RAG paper ID stored in the item's Extra field */
export function getRagPaperId(item: Zotero.Item): string | null {
  const extra = (item.getField("extra") as string) || "";
  for (const line of extra.split("\n")) {
    if (line.startsWith(RAG_ID_PREFIX)) {
      return line.slice(RAG_ID_PREFIX.length).trim();
    }
  }
  return null;
}

/** Store the RAG paper ID in the item's Extra field */
export async function setRagPaperId(item: Zotero.Item, paperId: string): Promise<void> {
  const extra = (item.getField("extra") as string) || "";
  const lines = extra.split("\n").filter((l) => !l.startsWith(RAG_ID_PREFIX));
  lines.push(`${RAG_ID_PREFIX}${paperId}`);
  item.setField("extra", lines.join("\n"));
  await item.saveTx();
}

/** Remove the RAG paper ID from the item's Extra field */
export async function clearRagPaperId(item: Zotero.Item): Promise<void> {
  const extra = (item.getField("extra") as string) || "";
  const lines = extra.split("\n").filter((l) => !l.startsWith(RAG_ID_PREFIX));
  item.setField("extra", lines.filter(Boolean).join("\n"));
  await item.saveTx();
}
