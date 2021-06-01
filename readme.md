# Niu Language

Niu言語は, 競技プログラミングにおけるライブラリ制作のための言語です. 開発段階なので注意してください.

## 背景

競技プログラミングではC++を使う人口が多いです. 理由としては, 

- 競技中は型について気を配る時間がなく, 暗黙の型変換で済んでしまう
- 破壊的代入をしやすい
- 標準ライブラリが豊富
- それなりの実行速度がある

また, 競技プログラミングでもC++に代わる言語としてRustが注目され始めています. 
こちらは競技プログラミングの中でもライブラリ(コンテスト前にあらかじめ用意しておく, よく使うデータ構造・アルゴリズムを実装しておいたもの)の制作に向いています.

- トレイト制約による可読性のあるコードやコンパイルエラー表示
- デフォルトでmoveするので, 気づかないデータのコピーが起こらない
- ライフタイムによる参照切れのチェック
- enum, matchなどの構文
- ブロック文が値を返すことができる
- C++と同等の実行速度

使いたい言語がふたつあると, コンテスト用と観賞用の2種類のライブラリを書かなければいけません.  
「ライブラリはRustで書きたいけど, 競技プログラミングのコンテストはC++で出たい」これを解決したいと思い, 開発し始めたのがNiu言語です.  

- ライブラリ制作時は, Niu言語を使ってデータ構造・アルゴリズムを実装
- コンテスト前にライブラリをC++にトランスパイル
- コンテスト中はコピペで利用

が目標です.

## 盛り込みたい機能

- C++のためのアノテーションを直す
  - 下のようにトランスパイルする(C++で壊れないように)
  - Niu: `let var = func(1, false)`
  - C++: `int var = func<int, bool>(1, false)`
- 構造体(Generics)
- C++からの構造体・関数のインポート
  - 例: `std::vector`
  - Niu言語独自に`Vec`を作ってしまうと, 提出コードの肥大化が起こってしまう.
  - Niu言語では`Vec`, トランスパイルしたC++のコードでは`std::vector`を使うコードに変換できればうれしい
- ポインタ, 参照

## 現時点の機能

### literal

- `u64`: 数字 (+ `u64`)
- `i64`: 数字 + `i64`
- `bool`: `true`か`false` 

### Operator

- `Or`
- `And`
- `Ord`: 二項演算のみ(`a == b == c`はエラー)
- `BitOr`
- `BitXor`
- `BitAnd`
- `Shift`
- `Add`, `Sub`
- `Mul`, `Div`, `Rem`

今のところ演算に関する型チェックは, 左辺と右辺の型が同じであることしか確認してない.  
`Or`, `And`, `Ord`については, 型が`bool`であるかのチェックも行っている.

### Block

`let var = { let a = 1 + 2; a * 2 };`  
`void`に対応してないので返す値がないと壊れます

### Method

`fn func(a: i64 , b: i64) -> i64 { a + b }
`void`に対応してないので返す値がないと壊れます

### Generics Method

`fn func<T>(a: T) -> T { a }`

### Trait

```
trait MyTrait {
  type Associated;
  fn func(t: Self) -> Self#MyTrait::Associated;
}

impl MyTrait for i64 {
  type Associated = u64;
  fn func(t: i64) -> i64#MyTrait::Associated {
    1u64
  }
}

fn apply<T: MyTrait>(t: T) -> T#MyTrait::Associated {
  T#MyTrait.func(t)
}

fn apply_for_i64(t: i64) -> T#MyTrait::Associated {
  apply(t)
}
```

### Struct

```
struct Pair {
  a: i64,
  b: i64,
}

trait Sum {
  type Output;
  fn sum(self: Self) -> Self#Sum::Output;
}

impl Sum for Pair {
  type Output = i64;
  fn sum(self: Pair) -> Pair#Sum::Output {
    self.a + self.b
  }
}

fn apply() -> i64 {
  let p = Pair { a: 9i64, b: 1i64, };
  Pair#Sum.sum(p)
}

```

```
struct Pair<S, T> {
  a: S,
  b: T,
}

fn get_a(p: Pair<i64, u64>) -> i64 {
  p.a
}

fn get_b(p: Pair<i64, u64>) -> u64 {
  p.b
}

fn generics_get_a<S, T>(p: Pair<S, T>) -> S {
  p.a
}

fn generics_get_b<S, T>(p: Pair<S, T>) -> T {
  p.b
}

fn generics_new<S, T>(s: S, t: T) -> Pair<S, T> {
  let p = Pair { a: s, b: t };
  p
}
```


## 遊び方

```
cargo run test.niu
```

## 注意

- ある条件で型チェックが無限ループする
- Trait用のために改修したのでトランスパイルしたあとのC++のアノテーションの部分が壊れるかも
