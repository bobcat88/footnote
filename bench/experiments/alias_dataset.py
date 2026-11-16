"""Footnote Alias evaluation set v1 (ALIAS-EVAL.md): French and English business texts with personal data spans.

    bench/.labenv/Scripts/python.exe -I bench/experiments/alias_dataset.py

Writes bench/datasets/alias-v1/{dev,heldout}.jsonl. Every value is fictional: names from common-name lists, documentation IP
ranges, RFC 2606 e-mail domains, phone numbers in reserved fiction ranges where they exist, and French identifiers built with valid
check digits (NIR key, SIREN/SIRET Luhn, IBAN mod 97, card Luhn) so a checksum rule must accept them. Spans are recorded while the
text is assembled, so annotations cannot drift from the text. The split is by template: no template appears in both halves.
"""
from __future__ import annotations

import json
import random
import re
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "datasets/alias-v1"
SEED = 20261116
PER_TEMPLATE = 6

FIRST_FR = ["Marie", "Camille", "Léa", "Chloé", "Manon", "Inès", "Julien", "Thomas", "Nicolas", "Hugo", "Maxime", "Antoine",
            "Élodie", "Sébastien", "François", "Hélène", "Gaëlle", "Loïc", "Mathis", "Anaïs"]
LAST_FR = ["Martin", "Bernard", "Dubois", "Lefèvre", "Moreau", "Laurent", "Girard", "Roux", "Fournier", "Morel", "Le Goff",
           "Kerbrat", "Guillou", "Rousseau", "Blanc", "Garnier", "Faure", "Chevalier", "Lemaire", "Benali"]
FIRST_EN = ["James", "Emily", "Olivia", "Daniel", "Sophie", "Michael", "Grace", "Ethan", "Hannah", "Lucas", "Amelia",
            "Noah", "Charlotte", "Oliver", "Priya", "Aisha", "Liam", "Chloe", "Samuel", "Megan"]
LAST_EN = ["Smith", "Johnson", "Taylor", "Brown", "Williams", "Wilson", "Davies", "Evans", "Thomas", "Roberts", "Walker",
           "Wright", "Patel", "Khan", "Murphy", "O'Brien", "Hughes", "Clarke", "Turner", "Baker"]
STREETS_FR = ["rue des Lilas", "avenue Jean Jaurès", "boulevard de la Liberté", "rue du Port", "impasse des Tilleuls",
              "place de la Mairie", "rue Saint-Michel", "chemin des Vignes"]
CITIES_FR = [("29200", "Brest"), ("35000", "Rennes"), ("69003", "Lyon"), ("44000", "Nantes"), ("75011", "Paris"),
             ("33000", "Bordeaux"), ("31000", "Toulouse"), ("59000", "Lille")]
STREETS_EN = ["Baker Street", "Elm Road", "Market Lane", "Church Street", "Victoria Avenue", "Mill Road", "Station Road"]
CITIES_EN = [("Manchester", "M1 4BT"), ("Leeds", "LS1 3AB"), ("Bristol", "BS1 5TR"), ("York", "YO1 7HH")]
MONTHS_FR = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre",
             "décembre"]
MONTHS_EN = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
             "December"]


def luhn_ok(digits: str) -> bool:
    total = 0
    for i, ch in enumerate(reversed(digits)):
        d = int(ch) * (2 if i % 2 else 1)
        total += d - 9 if d > 9 else d
    return total % 10 == 0


def luhn_complete(prefix: str) -> str:
    return next(prefix + str(c) for c in range(10) if luhn_ok(prefix + str(c)))


def iban_mod97(iban: str) -> int:
    s = iban[4:] + iban[:4]
    return int("".join(str(int(c, 36)) for c in s)) % 97


class Gen:
    def __init__(self, rng: random.Random) -> None:
        self.r = rng

    def digits(self, n: int) -> str:
        return "".join(self.r.choice("0123456789") for _ in range(n))

    def person(self, lang: str) -> str:
        f, l = (FIRST_FR, LAST_FR) if lang == "fr" else (FIRST_EN, LAST_EN)
        return f"{self.r.choice(f)} {self.r.choice(l)}"

    def first(self, lang: str) -> str:
        return self.r.choice(FIRST_FR if lang == "fr" else FIRST_EN)

    def email(self, lang: str) -> str:
        f, l = (FIRST_FR, LAST_FR) if lang == "fr" else (FIRST_EN, LAST_EN)
        plain = lambda s: re.sub(r"[^a-z]", "", s.lower().translate(str.maketrans("éèêëàâçîïôöùûü", "eeeeaaciioouuu")))  # noqa: E731
        dom = self.r.choice(["example.com", "example.org", "example.net", "mail.example.com", "acme.example.org"])
        return f"{plain(self.r.choice(f))}.{plain(self.r.choice(l))}@{dom}"

    def phone(self, lang: str) -> str:
        if lang == "fr":
            n = self.r.choice(["6", "7", "1", "2"]) + self.digits(8)
            pairs = " ".join(n[i:i + 2] for i in range(1, 9, 2))
            return self.r.choice([f"0{n[0]} {pairs}", f"+33 {n[0]} {pairs}", f"0{n[0]}{n[1:]}"])
        return self.r.choice([f"+44 20 7946 0{self.digits(3)}", f"(555) 01{self.digits(2)}-{self.digits(4)}",
                              f"07700 900{self.digits(3)}"])  # Ofcom and NANP fiction ranges

    def address(self, lang: str) -> str:
        n = self.r.randint(1, 120)
        if lang == "fr":
            cp, city = self.r.choice(CITIES_FR)
            return f"{n} {self.r.choice(STREETS_FR)}, {cp} {city}"
        city, pc = self.r.choice(CITIES_EN)
        return f"{n} {self.r.choice(STREETS_EN)}, {city} {pc}"

    def dob(self, lang: str) -> str:
        d, m, y = self.r.randint(1, 28), self.r.randint(1, 12), self.r.randint(1950, 2004)
        if lang == "fr":
            return self.r.choice([f"{d:02d}/{m:02d}/{y}", f"{d} {MONTHS_FR[m - 1]} {y}"])
        return self.r.choice([f"{MONTHS_EN[m - 1]} {d}, {y}", f"{d:02d}/{m:02d}/{y}"])

    def nir(self) -> str:
        sex, yy, mm = self.r.choice("12"), f"{self.r.randint(50, 99):02d}", f"{self.r.randint(1, 12):02d}"
        dept = self.r.choice(["29", "35", "75", "69", "2A", "2B", "44"])
        rest = self.digits(6)
        numeric = f"{sex}{yy}{mm}{dept.replace('2A', '19').replace('2B', '18')}{rest}"
        key = 97 - int(numeric) % 97
        raw = f"{sex}{yy}{mm}{dept}{rest}"
        return self.r.choice([f"{raw} {key:02d}", f"{raw[0]} {raw[1:3]} {raw[3:5]} {raw[5:7]} {raw[7:10]} {raw[10:]} {key:02d}"])

    def siren(self) -> str:
        return luhn_complete(self.r.choice("3456789") + self.digits(7))

    def siret(self) -> str:
        s = luhn_complete(self.siren() + self.digits(4))
        return self.r.choice([s, f"{s[:3]} {s[3:6]} {s[6:9]} {s[9:]}"])

    def iban(self) -> str:
        bank, branch, account = self.digits(5), self.digits(5), self.digits(11)
        rib = 97 - (89 * int(bank) + 15 * int(branch) + 3 * int(account)) % 97
        bban = f"{bank}{branch}{account}{rib:02d}"
        check = 98 - iban_mod97(f"FR00{bban}")
        iban = f"FR{check:02d}{bban}"
        assert iban_mod97(iban) == 1
        return self.r.choice([iban, " ".join(iban[i:i + 4] for i in range(0, len(iban), 4))])

    def card(self) -> str:
        prefix = self.r.choice(["4", "51", "53", "55"])
        c = luhn_complete(prefix + self.digits(15 - len(prefix)))
        return self.r.choice([c, " ".join(c[i:i + 4] for i in range(0, 16, 4))])

    def ip(self) -> str:
        return f"{self.r.choice(['192.0.2', '198.51.100', '203.0.113'])}.{self.r.randint(1, 254)}"

    # Look-alikes that must NOT be tagged.
    def order(self) -> str:
        return f"CMD-2026-{self.digits(5)}"

    def bad_card(self) -> str:  # 16 digits, Luhn deliberately wrong: a tracking number, not a card
        c = luhn_complete("1" + self.digits(14))
        return c[:-1] + str((int(c[-1]) + 1) % 10)

    def amount(self) -> str:
        return f"{self.r.randint(120, 98_000):,}".replace(",", " ") + " €"


LABELS = {"PERSON", "FIRST", "EMAIL", "PHONE", "ADDRESS", "DOB", "NIR", "SIREN", "SIRET", "IBAN", "CARD", "IP"}
TRAPS = {"ORDER", "TRACKING", "AMOUNT", "COMPANY", "VERSION", "MEETING_DATE"}

# Templates: {LABEL} is personal data, {trap:KIND} is a look-alike that must stay untouched. FIRST is a first name alone.
TEMPLATES = {
    "fr": [
        "Bonjour {FIRST}, suite à notre appel, pouvez-vous me renvoyer le RIB ? L'IBAN actuel {IBAN} est refusé. Cordialement, {PERSON}",
        "Note CRM : {PERSON} (né le {DOB}) souhaite résilier. Joignable au {PHONE} ou par mail {EMAIL}.",
        "Ticket #{trap:ORDER} : le client {PERSON} signale un double prélèvement de {trap:AMOUNT} sur la carte {CARD}.",
        "RH – arrêt maladie de {PERSON}, n° de sécurité sociale {NIR}, à transmettre à la mutuelle avant vendredi.",
        "Facture fournisseur {trap:COMPANY}, SIRET {SIRET}, montant {trap:AMOUNT}, contact comptable {EMAIL}.",
        "Compte rendu : {PERSON} et {PERSON} ont validé la version {trap:VERSION} lors du point du {trap:MEETING_DATE}.",
        "Merci d'envoyer le contrat à {PERSON}, {ADDRESS}, et de prévenir {FIRST} par téléphone au {PHONE}.",
        "Alerte sécurité : connexion inhabituelle au compte de {PERSON} depuis l'adresse {IP}, colis suivi {trap:TRACKING}.",
        "Le prestataire {trap:COMPANY} (SIREN {SIREN}) facture {trap:AMOUNT} ; référence commande {trap:ORDER}.",
        "Candidature reçue : {PERSON}, {DOB}, demeurant {ADDRESS}. Téléphone : {PHONE}.",
        "Salut {FIRST}, tu peux regarder pourquoi le virement vers {IBAN} n'est pas parti ? Merci !",
        "Réclamation de {PERSON} : la livraison {trap:TRACKING} n'est jamais arrivée au {ADDRESS}.",
        "Pour le dossier de prêt, joindre la pièce d'identité de {PERSON} et son numéro de sécurité sociale {NIR}.",
        "Réunion du {trap:MEETING_DATE} : rien à signaler, la version {trap:VERSION} passe en production chez {trap:COMPANY}.",
        "{PERSON} a appelé du {PHONE} pour changer son adresse e-mail en {EMAIL}.",
        "Paiement refusé pour la carte {CARD} de {PERSON} ; montant {trap:AMOUNT}, commande {trap:ORDER}.",
        "Bonjour, je suis {PERSON}, mon numéro client est {trap:ORDER} et j'habite {ADDRESS}.",
        "Le serveur {IP} a été bloqué après trois échecs de connexion de l'utilisateur {EMAIL}.",
        "Mutation : {PERSON} rejoint l'agence de {trap:COMPANY} ; mettre à jour son RIB {IBAN} dans la paie.",
        "Rappel : {FIRST} {trap:MEETING_DATE} pour la revue, apporter les chiffres de {trap:COMPANY}.",
        "Déclaration d'accident du travail de {PERSON}, né le {DOB}, NIR {NIR}, employeur SIRET {SIRET}.",
        "Point budget : {trap:AMOUNT} engagés sur la version {trap:VERSION}, aucun dépassement signalé.",
        "Merci {FIRST} pour ton aide hier ! Je t'envoie le fichier ce soir sur {EMAIL}.",
        "Suspicion de fraude : carte {CARD} utilisée depuis {IP} pour la commande {trap:ORDER}.",
        "Le colis {trap:TRACKING} de {PERSON} est en attente au relais, contact {PHONE}.",
        "Kbis reçu pour {trap:COMPANY} : SIREN {SIREN}, gérant {PERSON}, joignable au {PHONE}.",
    ],
    "en": [
        "Hi {FIRST}, following our call, could you resend the bank details? The IBAN {IBAN} was rejected. Best, {PERSON}",
        "CRM note: {PERSON} (DOB {DOB}) wants to cancel. Reach them on {PHONE} or at {EMAIL}.",
        "Ticket #{trap:ORDER}: customer {PERSON} reports a double charge of {trap:AMOUNT} on card {CARD}.",
        "HR: sick leave for {PERSON}, social security number {NIR}, to send to the insurer by Friday.",
        "Supplier invoice from {trap:COMPANY}, SIRET {SIRET}, amount {trap:AMOUNT}, accounts contact {EMAIL}.",
        "Minutes: {PERSON} and {PERSON} approved version {trap:VERSION} at the {trap:MEETING_DATE} review.",
        "Please post the contract to {PERSON}, {ADDRESS}, and give {FIRST} a call on {PHONE}.",
        "Security alert: unusual sign-in to {PERSON}'s account from {IP}; parcel {trap:TRACKING} flagged too.",
        "Vendor {trap:COMPANY} (SIREN {SIREN}) bills {trap:AMOUNT}; purchase order {trap:ORDER}.",
        "Application received: {PERSON}, born {DOB}, living at {ADDRESS}. Phone: {PHONE}.",
        "Hey {FIRST}, can you check why the transfer to {IBAN} didn't go through? Thanks!",
        "Complaint from {PERSON}: delivery {trap:TRACKING} never reached {ADDRESS}.",
        "For the loan file, attach {PERSON}'s ID and their French social security number {NIR}.",
        "Meeting on {trap:MEETING_DATE}: nothing to report, version {trap:VERSION} ships to {trap:COMPANY}.",
        "{PERSON} called from {PHONE} to change their email to {EMAIL}.",
        "Payment declined for {PERSON}'s card {CARD}; amount {trap:AMOUNT}, order {trap:ORDER}.",
        "Hello, I'm {PERSON}, my customer number is {trap:ORDER} and I live at {ADDRESS}.",
        "Host {IP} was blocked after three failed logins by user {EMAIL}.",
        "Transfer: {PERSON} joins the {trap:COMPANY} branch; update their bank account {IBAN} in payroll.",
        "Reminder: {FIRST}, review on {trap:MEETING_DATE}, bring the {trap:COMPANY} figures.",
        "Workplace accident report for {PERSON}, born {DOB}, NIR {NIR}, employer SIRET {SIRET}.",
        "Budget check: {trap:AMOUNT} committed on version {trap:VERSION}, no overrun reported.",
        "Thanks {FIRST} for your help yesterday! I'll send the file tonight to {EMAIL}.",
        "Fraud suspicion: card {CARD} used from {IP} for order {trap:ORDER}.",
        "Parcel {trap:TRACKING} for {PERSON} is waiting at the pick-up point, contact {PHONE}.",
        "Company record received for {trap:COMPANY}: SIREN {SIREN}, director {PERSON}, reachable on {PHONE}.",
    ],
}
COMPANIES = ["Acme Industrie", "Globex", "Initech", "Société Exemple", "Umbrella Services", "Stark Conseil"]


def fill(template: str, lang: str, g: Gen) -> dict:
    text, spans, traps, pos = "", [], [], 0
    for m in re.finditer(r"\{(trap:)?([A-Z_]+)\}", template):
        text += template[pos:m.start()]
        kind = m.group(2)
        if m.group(1):
            value = {"ORDER": g.order, "TRACKING": g.bad_card, "AMOUNT": g.amount, "VERSION": lambda: f"{g.r.randint(1, 9)}.{g.r.randint(0, 20)}.{g.r.randint(0, 9)}",
                     "COMPANY": lambda: g.r.choice(COMPANIES), "MEETING_DATE": lambda: g.dob(lang).replace("19", "20", 1)}[kind]()
            traps.append({"start": len(text), "end": len(text) + len(value), "kind": kind})
        else:
            value = {"PERSON": lambda: g.person(lang), "FIRST": lambda: g.first(lang), "EMAIL": lambda: g.email(lang),
                     "PHONE": lambda: g.phone(lang), "ADDRESS": lambda: g.address(lang), "DOB": lambda: g.dob(lang),
                     "NIR": g.nir, "SIREN": g.siren, "SIRET": g.siret, "IBAN": g.iban, "CARD": g.card, "IP": g.ip}[kind]()
            spans.append({"start": len(text), "end": len(text) + len(value), "label": "PERSON" if kind == "FIRST" else kind})
        text += value
        pos = m.end()
    text += template[pos:]
    return {"text": text, "spans": spans, "traps": traps}


def check(row: dict) -> None:
    """Self-check: every span covers its value, identifiers carry valid check digits, look-alike cards fail Luhn."""
    for s in row["spans"]:
        v = row["text"][s["start"]:s["end"]]
        assert v and v == v.strip(), row
        d = re.sub(r"\D", "", v)
        if s["label"] in ("CARD", "SIREN", "SIRET"):
            assert luhn_ok(d), (s, v)
        if s["label"] == "IBAN":
            assert iban_mod97(v.replace(" ", "")) == 1, v
        if s["label"] == "NIR":
            raw = v.replace(" ", "")
            body = raw[:13].replace("2A", "19").replace("2B", "18")
            assert 97 - int(body) % 97 == int(raw[13:]), v
    for t in row["traps"]:
        if t["kind"] == "TRACKING":
            assert not luhn_ok(row["text"][t["start"]:t["end"]])


def main() -> None:
    rng = random.Random(SEED)
    g = Gen(rng)
    OUT.mkdir(parents=True, exist_ok=True)
    halves = {"dev": [], "heldout": []}
    for lang, templates in TEMPLATES.items():
        kinds = [set(re.findall(r"\{([A-Z]+)\}", t)) - {"FIRST"} for t in templates]
        every = set().union(*kinds)
        order = list(range(len(templates)))
        for _ in range(10_000):  # redraw until each half has every label, so no label is judged on one half only
            rng.shuffle(order)
            dev = set(order[: len(order) // 2])
            if all(set().union(*(kinds[t] for t in half)) == every for half in (dev, set(order) - dev)):
                break
        else:
            raise SystemExit(f"{lang}: some label has a single template; add another")
        for t, template in enumerate(templates):
            for k in range(PER_TEMPLATE):
                row = {"id": f"{lang}-{t:02d}-{k}", "lang": lang, "template": t, **fill(template, lang, g)}
                check(row)
                halves["dev" if t in dev else "heldout"].append(row)
    for name, rows in halves.items():
        (OUT / f"{name}.jsonl").write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")
        labels: dict[str, int] = {}
        for r in rows:
            for s in r["spans"]:
                labels[s["label"]] = labels.get(s["label"], 0) + 1
        print(name, len(rows), "texts", dict(sorted(labels.items())), "traps", sum(len(r["traps"]) for r in rows))


if __name__ == "__main__":
    main()
