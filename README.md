# property-manager

Outil en Rust pour gérer un portefeuille de biens immobiliers (parkings, appartements, etc.) : achats, dépenses, locations et suivi des loyers, avec stockage local via SQLite. Utilisable en ligne de commande ou via un dashboard interactif dans le terminal.

## Fonctionnalités

- **Gestion des biens** : enregistrement des achats (prix, date, adresse, notes)
- **Suivi des dépenses** : taxes, réparations, charges — ponctuelles ou récurrentes
  - **Frais directs** : rattachés à un seul bien (ex. réparation d'une barrière)
  - **Frais indirects** : partagés entre plusieurs biens (ex. syndic, fisc), répartis automatiquement à parts égales
- **Gestion locative** : locataires, baux (actifs ou terminés), historique des loyers encaissés
- **Rentabilité** : calcul automatique loyers encaissés − dépenses (directes + part des indirectes), par bien ou pour tout le portefeuille
- **Détection des loyers impayés** : identifie les mois sans paiement enregistré pour chaque bail actif
- **Intégrité des données** : suppression d'un bien bloquée tant que des baux ou dépenses y sont rattachés ; montants et dates validés à la construction et en base (voir *Invariants et validation* ci-dessous)
- **CLI complète** : ajout et consultation de biens, locataires, baux, dépenses et paiements
- **Dashboard terminal** : vue d'ensemble du portefeuille et détail par bien, navigable au clavier
- **Données de démonstration** : la base est peuplée automatiquement d'un jeu de données réaliste lors de sa toute première création

## Stack technique

- [Rust](https://www.rust-lang.org/) (édition 2024)
- [`rusqlite`](https://crates.io/crates/rusqlite) (feature `bundled`) — accès SQLite sans dépendance système
- [`chrono`](https://crates.io/crates/chrono) — gestion des dates
- [`thiserror`](https://crates.io/crates/thiserror) — erreurs applicatives typées
- [`clap`](https://crates.io/crates/clap) (feature `derive`) — interface en ligne de commande
- [`ratatui`](https://crates.io/crates/ratatui) + [`crossterm`](https://crates.io/crates/crossterm) — dashboard interactif dans le terminal

## Structure du projet

```
property-manager/
├── Cargo.toml
├── src/
│   ├── lib.rs              # point d'entrée de la librairie
│   ├── main.rs             # binaire : parsing CLI (clap) et dispatch des commandes
│   ├── error.rs            # type d'erreur applicatif (AppError / AppResult)
│   ├── db/
│   │   ├── mod.rs          # connexion SQLite, PRAGMA, ouverture de la base, déclenchement du seed
│   │   ├── schema.rs       # définition des tables, contraintes CHECK et index (migrations simples)
│   │   ├── seed.rs         # jeu de données de démonstration (base tout juste créée uniquement)
│   │   ├── repository.rs   # CRUD par table + répartition des frais indirects
│   │   ├── repository/
│   │   │   └── tests.rs
│   │   ├── reporting.rs    # requêtes composites : rentabilité, loyers en retard, détail par bien
│   │   └── reporting/
│   │       └── tests.rs
│   ├── models/              # champs privés, construction validée (::new / ::new_direct / ::new_indirect)
│   │   ├── property.rs
│   │   ├── expense.rs       # Expense + ExpenseTarget (Direct { property_id } / Indirect)
│   │   ├── tenant.rs
│   │   ├── lease.rs
│   │   └── rent_payment.rs
│   └── tui/
│       ├── mod.rs          # boucle d'événements, gestion du terminal, DashboardState (avec cache)
│       └── ui.rs           # construction des widgets (onglets, tableaux, panneaux)
└── tests/
    ├── property_lifecycle.rs   # scénario complet : achat → bail → loyers → dépenses
    └── reporting.rs            # scénario multi-biens : rentabilité et retards de paiement
```

## Modèle de données

| Table                | Description                                                        |
|-----------------------|----------------------------------------------------------------------|
| `property`           | Un bien immobilier : libellé, adresse, date et prix d'achat         |
| `expense`            | Une dépense : directe (`property_id` renseigné) ou indirecte (`property_id` NULL, répartie via `expense_allocation`) |
| `expense_allocation` | La part d'un frais indirect attribuée à un bien donné               |
| `tenant`             | Un locataire                                                        |
| `lease`              | Un bail : bien + locataire + loyer mensuel + dates de début/fin     |
| `rent_payment`       | Un paiement de loyer, rattaché à un bail et à une période (`YYYY-MM`) |

Tous les montants sont stockés en **centimes** (`INTEGER`), jamais en `REAL`, pour éviter les erreurs d'arrondi sur des données financières. Les dates sont stockées en `TEXT` au format ISO 8601 (`YYYY-MM-DD`), ce qui permet un tri lexicographique correct directement en SQL.

### Frais directs vs indirects

Un **frais direct** (`expense_type = 'direct'`) concerne un seul bien : `property_id` est renseigné directement sur la ligne `expense`. Un **frais indirect** (`expense_type = 'indirect'`) concerne plusieurs biens à la fois (charges de syndic communes, taxe partagée…) : `property_id` vaut `NULL` sur la ligne `expense`, et le montant est réparti à **parts égales** entre les biens concernés via la table `expense_allocation` — une ligne par bien, avec sa part en centimes. La répartition garantit que la somme des parts reconstitue exactement le montant total, y compris quand celui-ci ne se divise pas proprement (le reste de la division entière est distribué aux premières parts).

Côté Rust, cette distinction est portée par un type `ExpenseTarget` (`Direct { property_id: i64 }` ou `Indirect`) plutôt que par deux champs séparés — les quatre combinaisons théoriques (direct+id, direct+NULL, indirect+id, indirect+NULL) se réduisent ainsi aux deux seules valides, qu'il est impossible de construire de travers.

## Invariants et validation

Chaque invariant métier est appliqué à deux niveaux : à la construction des modèles Rust (retour `AppResult`, échec explicite plutôt que valeur silencieusement incohérente) et en base via des contraintes SQL, pour se protéger aussi d'un accès direct ou d'un futur bug de conversion.

| Invariant | Validation Rust | Contrainte SQL |
|---|---|---|
| Montants jamais négatifs (achat, dépense, loyer, part allouée) | `AppError::InvalidAmount` dans chaque constructeur (`Property::new`, `Expense::new_direct`/`new_indirect`, `Lease::new`, `RentPayment::new`) | `CHECK (... >= 0)` sur `purchase_price_cents`, `amount_cents`, `monthly_rent_cents` |
| Cohérence type de frais / bien associé | Impossible à construire de travers via `ExpenseTarget` | `CHECK` combinant `expense_type` et `property_id` sur `expense` |
| Répartition d'un frais indirect sans bien en double | `AppError::DuplicatePropertyAllocation` dans `insert_indirect_expense` | — (vérifié uniquement côté application) |
| Dates de bail cohérentes (fin ≥ début) | `AppError::InvalidLeaseDates` dans `Lease::new` | `CHECK (end_date IS NULL OR end_date >= start_date)` |
| Un seul bail actif par bien | `AppError::PropertyAlreadyHasActiveLease`, déduite d'une violation d'index en base | Index unique partiel `idx_one_active_lease_per_property` sur `lease(property_id) WHERE end_date IS NULL` |
| Suppression d'un bien encore référencé | `AppError::PropertyHasDependents`, déduite d'une violation de clé étrangère | `PRAGMA foreign_keys = ON` (par connexion, activé dans `db::open`) |

## Installation

Prérequis : [Rust](https://www.rust-lang.org/tools/install) (édition 2024 ou plus récente).

```bash
git clone <url-du-repo>
cd property-manager
cargo build
```

Aucune installation de SQLite n'est nécessaire : la feature `bundled` de `rusqlite` compile SQLite directement avec le projet.

Au tout premier lancement (fichier `property_manager.db` absent), la base est créée, migrée, puis peuplée automatiquement avec un jeu de données de démonstration : 4 biens (dont un vacant), 3 locataires, des baux actifs et un terminé, des loyers avec un mois volontairement manquant (pour illustrer la détection de retard), et des dépenses directes et indirectes. Les lancements suivants réutilisent la base existante sans la re-peupler.

## Utilisation

Le chemin de la base peut être changé avec `--db-path`.

### Gestion des biens

```bash
cargo run -- add-property "Parking A12" "Rue de la Gare 10" 2024-01-15 15000.00
cargo run -- list-properties
cargo run -- delete-property 1
```

### Locataires et baux

```bash
cargo run -- add-tenant "Jean Dupont" --contact jean@example.com
cargo run -- add-lease <property_id> <tenant_id> 80.00 2024-02-01
cargo run -- list-active-leases
```

### Dépenses et paiements

```bash
# Frais direct : rattaché à un seul bien
cargo run -- add-expense <property_id> "réparation barrière" 150.00 2024-06-01

# Frais indirect : réparti à parts égales sur plusieurs biens
cargo run -- add-indirect-expense "syndic" 100.01 2024-03-01 --properties 1,2,3

cargo run -- list-expenses <property_id>
cargo run -- add-payment <lease_id> 80.00 2024-02-03 2024-02
```

### Rapports

```bash
cargo run -- profitability
cargo run -- overdue
cargo run -- overdue --up-to 2024-06-30
```

### Dashboard interactif

```bash
cargo run -- dashboard
```

Affiche une vue d'ensemble du portefeuille (rentabilité par bien, loyers en retard) ainsi qu'un onglet détaillé par bien, avec :
- un résumé (loyers encaissés, dépenses, net, statut de paiement du bail actif)
- le détail des dépenses (directes et indirectes) et des loyers encaissés, affichés côte à côte

Navigation :

| Touche    | Action                                |
|-----------|-----------------------------------------|
| `←` / `→` | Changer d'onglet                        |
| `r`       | Recharger les données depuis la base    |
| `q`       | Quitter                                 |

Les montants saisis en ligne de commande sont exprimés en **euros** (ex. `15000.00`) et convertis automatiquement en centimes pour le stockage.

## Tests

```bash
cargo test
```

La suite de tests couvre :
- les opérations CRUD de chaque modèle, y compris la répartition des frais indirects et les invariants (montants, dates, unicité du bail actif) — `src/db/repository/tests.rs`
- les calculs de rentabilité et de loyers en retard — `src/db/reporting/tests.rs`
- des scénarios de bout en bout avec plusieurs biens dans des situations variées (`tests/property_lifecycle.rs`, `tests/reporting.rs`)

## Choix de conception

- **`Option<i64>` pour les identifiants** : un `Property` construit en mémoire (`Property::new`) n'a pas encore d'`id` (`None`) ; un `Property` lu depuis la base en a toujours un (`Some`). Le type rend cet état impossible à confondre.
- **Séparation `repository.rs` / `reporting.rs`** : le repository ne fait que du CRUD table par table (et la répartition des frais indirects, qui reste une opération d'écriture sur une seule table logique) ; `reporting.rs` regroupe les requêtes qui combinent plusieurs tables ou contiennent de la logique métier (calcul des mois de loyer manquants, détail agrégé d'un bien).
- **Répartition égale avec gestion du reste** : diviser un montant en centimes par N ne tombe pas toujours juste (`10001 / 3 ≠` un nombre entier de centimes) ; le reste de la division entière est distribué aux premières parts pour garantir que la somme des parts égale toujours exactement le montant total.
- **Erreurs typées (`AppError`)** : les erreurs SQLite, de parsing de date et d'I/O terminal sont converties en erreurs explicites (`PropertyNotFound`, `PropertyHasDependents`, `EmptyAllocation`, `Terminal`, etc.) plutôt que de laisser fuiter les détails d'implémentation dans le reste de l'application.
- **Suppression protégée** : SQLite (avec `PRAGMA foreign_keys = ON`) refuse par défaut de supprimer un bien encore référencé par un bail ou une dépense ; cette contrainte est traduite en erreur `AppError::PropertyHasDependents` plutôt que de laisser remonter une erreur SQLite brute. En CLI, cette erreur est affichée sans interrompre le programme.
- **Dashboard à cache invalidé sur événement** : les données (vue d'ensemble et détail par bien) restent en mémoire tant que l'utilisateur n'appuie pas sur `r`, ne change pas d'onglet, ou ne quitte pas — la boucle de rendu (~4 fois par seconde pour rester réactive au clavier) ne déclenche donc aucune requête SQL tant que rien de pertinent n'a changé.
- **Modèles encapsulés** : les champs des structs (`Property`, `Expense`, `Lease`, etc.) sont privés, accessibles via des méthodes (`p.label()`, `e.amount_cents()`…) plutôt que directement — ça garantit qu'un `Property`/`Expense`/`Lease` en circulation dans le programme a toujours été validé à sa création, aucun code externe ne peut le remettre dans un état incohérent après coup.

## Feuille de route

- [ ] Répartition personnalisée des frais indirects (proportionnelle à la surface, par exemple, plutôt qu'égale uniquement)
- [ ] Export CSV des rapports de rentabilité
- [ ] Migrations versionnées (`rusqlite_migration`)
- [ ] Mini graphique (sparkline) de l'évolution des loyers dans le dashboard

## Licence

À définir...