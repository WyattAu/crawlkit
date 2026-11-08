-- §G9 follow-up: persist each page's og:image (resolved absolute URL) so
-- share previews render the observed image instead of a fixed placeholder.
ALTER TABLE pages ADD COLUMN IF NOT EXISTS og_image TEXT;
