module m;
  and   g1  ( y , a , b );
  buf   ( q , d );
  not   ( z , x );
  nand  g2  ( o , a , b , c );
endmodule
// expected -----
module m;
  and g1 (y, a, b);
  buf (q, d);
  not (z, x);
  nand g2 (o, a, b, c);
endmodule
