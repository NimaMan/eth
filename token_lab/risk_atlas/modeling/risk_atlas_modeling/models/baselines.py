"""Registry for first-pass baseline models."""

from __future__ import annotations

from sklearn.ensemble import HistGradientBoostingClassifier, HistGradientBoostingRegressor, RandomForestClassifier, RandomForestRegressor
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LogisticRegression, Ridge
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.tree import DecisionTreeClassifier, DecisionTreeRegressor


def classification_models() -> dict[str, object]:
    return {
        "logistic_regression": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("scale", StandardScaler()),
            ("model", LogisticRegression(max_iter=1000, class_weight="balanced")),
        ]),
        "decision_tree": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", DecisionTreeClassifier(max_depth=6, min_samples_leaf=50, class_weight="balanced", random_state=7)),
        ]),
        "random_forest": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", RandomForestClassifier(
                n_estimators=150,
                max_depth=8,
                min_samples_leaf=30,
                class_weight="balanced_subsample",
                random_state=7,
                n_jobs=-1,
            )),
        ]),
        "hist_gradient_boosting": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", HistGradientBoostingClassifier(max_iter=120, learning_rate=0.06, max_leaf_nodes=15, random_state=7)),
        ]),
    }


def regression_models() -> dict[str, object]:
    return {
        "ridge_log_time": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("scale", StandardScaler()),
            ("model", Ridge(alpha=1.0)),
        ]),
        "decision_tree_log_time": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", DecisionTreeRegressor(max_depth=6, min_samples_leaf=30, random_state=7)),
        ]),
        "random_forest_log_time": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", RandomForestRegressor(
                n_estimators=150,
                max_depth=8,
                min_samples_leaf=20,
                random_state=7,
                n_jobs=-1,
            )),
        ]),
        "hist_gradient_boosting_log_time": Pipeline([
            ("impute", SimpleImputer(strategy="median")),
            ("model", HistGradientBoostingRegressor(max_iter=120, learning_rate=0.06, max_leaf_nodes=15, random_state=7)),
        ]),
    }
