## improvements

- stoquer les résultats par dossier

actuel :

    "fichier 1": {
        "full_path",
        "type"
    },
    "fichier 2": {
        "full_path",
        "type"
    }

plus logique :

    [
        "dossier A": {
            "fichier 1": {
                "filename",
                "type"
            },
            "fichier 2": {
                "filename",
                "type"
            }
        }
    ]