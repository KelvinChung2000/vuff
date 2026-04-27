module m;
  initial begin
    foo (1, 2 ,3);
    bar(  );
    baz(  a + b , c );
  end
endmodule
// expected -----
module m;
  initial begin
    foo(1, 2, 3);
    bar();
    baz(a + b, c);
  end
endmodule
