# Contribuer

Les PR et forks gratuits sont les bienvenus. Toute contribution proposée pour inclusion relève de la licence du projet, sans transfert de copyright.

## Avant une PR

- Décrire le comportement avant/après et un scénario vérifiable.
- Ajouter un test pour toute correction de protocole, de timeline, de capacité ou de concurrence.
- Exécuter `cargo fmt --all -- --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, ainsi que build/lint/tests dans `ui`.
- Pour le média, joindre les résultats du banc local et préciser codecs, GOP, délai et limites. Ne pas utiliser de clé réelle ou diffuser sur une plateforme dans un test automatisé.
- Ne jamais capturer l'écran utilisateur dans les tests. Les snapshots éventuels doivent concerner une instance de développement isolée et être explicitement demandés.
- Ne pas incorporer de code, logos, captures ou assets extraits d'InstantDelay ; ne pas copier du code GPL incompatible avec la restriction de vente.
- Examiner les licences des nouvelles dépendances et régénérer SBOM/notices avant livraison.

Les artefacts sous `.local/`, `research-artifacts/`, les médias, jetons et données de session ne doivent pas être commités. Ne pas publier les preuves propriétaires de recherche.

Une implémentation partielle d'un codec reste indisponible ou clairement signalée ; aucun statut de protection ne doit provenir de la seule consigne. Une erreur de route ne doit pas être avalée ni affecter silencieusement les autres.
