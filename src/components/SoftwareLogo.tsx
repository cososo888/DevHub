interface LogoProps {
  id: string;
  name: string;
}

const text = (value: string, fill = "#fff", size = 18) => (
  <text x="32" y="34" textAnchor="middle" dominantBaseline="central" fill={fill} fontSize={size} fontWeight={800} fontFamily="-apple-system, BlinkMacSystemFont, sans-serif">{value}</text>
);

export function SoftwareLogo({ id, name }: LogoProps) {
  switch (id) {
    case "node":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M32 6 56 20v24L32 58 8 44V20z" fill="#5FA04E" /><path d="M32 12 50 22v20L32 52 14 42V22z" fill="#1D3B1B" opacity=".22" />{text("JS")}</svg>;
    case "python":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><rect x="9" y="9" width="24" height="38" rx="12" fill="#3776AB" /><rect x="31" y="17" width="24" height="38" rx="12" fill="#FFD43B" /><circle cx="21" cy="19" r="3" fill="#fff" /><circle cx="43" cy="45" r="3" fill="#3776AB" /></svg>;
    case "java":
    case "jdk":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M18 25h24v11a12 12 0 0 1-24 0z" fill="#E76F00" /><path d="M42 28h5a6 6 0 0 1 0 12h-5" fill="none" stroke="#E76F00" strokeWidth="4" /><path d="M25 15c-3-4 3-6 0-10M33 15c-3-4 3-6 0-10M41 15c-3-4 3-6 0-10" fill="none" stroke="#9AA4B2" strokeWidth="3" strokeLinecap="round" /><path d="M16 52h32" stroke="#E76F00" strokeWidth="4" strokeLinecap="round" /></svg>;
    case "go":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><circle cx="32" cy="32" r="26" fill="#EAF7FB" /><circle cx="23" cy="28" r="3" fill="#00ADD8" /><circle cx="41" cy="28" r="3" fill="#00ADD8" /><text x="32" y="43" textAnchor="middle" dominantBaseline="central" fill="#00ADD8" fontSize="16" fontWeight={800} fontFamily="-apple-system, BlinkMacSystemFont, sans-serif">GO</text></svg>;
    case "rust":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><circle cx="32" cy="32" r="22" fill="#CE422B" /><circle cx="32" cy="32" r="16" fill="none" stroke="#fff" strokeWidth="2" opacity=".65" />{text("R", "#fff", 22)}</svg>;
    case "git":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M20 20v18c0 5 4 9 9 9h15" fill="none" stroke="#F05032" strokeWidth="4" strokeLinecap="round" /><circle cx="20" cy="16" r="6" fill="#F05032" /><circle cx="20" cy="46" r="6" fill="#F05032" /><circle cx="46" cy="47" r="6" fill="#F05032" /></svg>;
    case "maven":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M12 54 32 8l20 46z" fill="#C71A36" /><path d="M22 46 32 24l10 22" fill="none" stroke="#fff" strokeWidth="4" strokeLinecap="round" />{text("M", "#fff", 11)}</svg>;
    case "gradle":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><circle cx="32" cy="32" r="22" fill="#02303A" /><path d="M17 35c11 12 27 7 31-8" fill="none" stroke="#7ED321" strokeWidth="6" strokeLinecap="round" />{text("G", "#fff", 15)}</svg>;
    case "mysql":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M14 18v28c0 4 8 7 18 7s18-3 18-7V18" fill="#00758F" /><ellipse cx="32" cy="18" rx="18" ry="7" fill="#0096B4" />{text("SQL", "#fff", 12)}</svg>;
    case "postgresql":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><circle cx="32" cy="32" r="24" fill="#336791" /><path d="M22 42c0-16 7-24 17-24 6 0 10 4 10 10v16" fill="none" stroke="#fff" strokeWidth="4" strokeLinecap="round" />{text("PG", "#fff", 13)}</svg>;
    case "redis":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><ellipse cx="32" cy="18" rx="20" ry="7" fill="#DC382D" /><path d="M12 18v10c0 4 9 7 20 7s20-3 20-7V18" fill="#B62B22" /><path d="M12 30v10c0 4 9 7 20 7s20-3 20-7V30" fill="#DC382D" /><path d="M12 42v6c0 4 9 7 20 7s20-3 20-7v-6" fill="#B62B22" /></svg>;
    case "docker":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><rect x="12" y="28" width="10" height="10" rx="2" fill="#2496ED" /><rect x="25" y="28" width="10" height="10" rx="2" fill="#2496ED" /><rect x="38" y="28" width="10" height="10" rx="2" fill="#2496ED" /><rect x="25" y="15" width="10" height="10" rx="2" fill="#2496ED" opacity=".8" /><path d="M8 42c8 8 28 10 38-2 5-6 9-4 10-4-1 12-12 22-27 22C17 58 9 51 8 42z" fill="#2496ED" /></svg>;
    case "vscode":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><path d="M45 9 20 32l25 23 8-4V13z" fill="#007ACC" /><path d="M20 32 8 22l5-5 13 11z" fill="#3FA9F5" /><path d="M20 32 8 42l5 5 13-11z" fill="#3FA9F5" />{text("VS", "#fff", 12)}</svg>;
    case "intellij":
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><rect x="8" y="8" width="48" height="48" rx="12" fill="#FE315D" /><rect x="14" y="44" width="26" height="5" rx="2" fill="#fff" opacity=".85" />{text("IJ", "#fff", 18)}</svg>;
    default:
      return <svg viewBox="0 0 64 64" className="software-logo" aria-hidden="true"><circle cx="32" cy="32" r="26" fill="#DDE3EE" />{text(name.slice(0, 2), "#4A5568", 13)}</svg>;
  }
}
